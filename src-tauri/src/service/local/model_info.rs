use std::{collections::HashSet, path::Path};

use anyhow::{bail, Context};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::de::DeserializeOwned;
use tokio::fs;

use crate::{
    common::local_paths::{ensure_child_dir, resolve_local_log_dir},
    config::HardwareType,
    service::{
        local::entity::model_info as model_info_entity,
        models::{ModelDownloadType, ModelInfo, ModelMutationResult},
        pipeline::{
            model_artifacts::{resolve_model_download_paths, validate_model_artifact_paths},
            script_paths::{
                resolve_src_model_root, src_model_begin_llm_task_script_path, ScriptPlatform,
            },
            validate_and_download, validate_and_init, PipelineBootstrapPaths,
            DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
        LocalService,
    },
    utils::{
        process::{run_logged_command, run_logged_command_with_output},
        time::now_string,
    },
    Result,
};

impl LocalService {
    pub(crate) async fn list_model_infos_impl(&self) -> Result<Vec<ModelInfo>> {
        let rows = model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .order_by_asc(model_info_entity::Column::Id)
            .all(self.orm())
            .await?;

        rows.into_iter().map(map_model_info).collect()
    }

    pub(crate) async fn model_downloaded_impl(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<bool> {
        Ok(self
            .find_model_info_row(base_model, model_version)
            .await?
            .map(|row| row.downloaded)
            .unwrap_or(false))
    }

    pub(crate) async fn set_model_downloaded_impl(
        &self,
        base_model: &str,
        model_version: &str,
        downloaded: bool,
    ) -> Result<()> {
        let Some(row) = self.find_model_info_row(base_model, model_version).await? else {
            return Ok(());
        };

        let mut active_model: model_info_entity::ActiveModel = row.into();
        active_model.downloaded = Set(downloaded);
        active_model.modify_time = Set(now_string()?);
        active_model.update(self.orm()).await?;

        Ok(())
    }

    pub(crate) async fn get_model_info_by_base_and_scale_impl(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<ModelInfo> {
        let row = self
            .find_model_info_row(base_model, model_version)
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到目标模型"))?;
        map_model_info(row)
    }

    async fn find_model_info_row(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<Option<model_info_entity::Model>> {
        model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .filter(model_info_entity::Column::BaseModel.eq(base_model.trim()))
            .filter(model_info_entity::Column::ModelVersion.eq(model_version.trim()))
            .one(self.orm())
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn install_model_impl(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelMutationResult> {
        let row = self.find_model_info_row_by_id(model_id).await?;
        let model_info = map_model_info(row.clone())?;
        if !model_info.supported_devices.contains(&device) {
            bail!(
                "模型 {} {} 不支持设备 {}，请切换为 {:?}",
                model_info.model_name,
                model_info.model_version,
                device,
                model_info.supported_devices
            );
        }
        let runtime_config = self.runtime_config()?;
        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let log_dir =
            ensure_child_dir(&resolve_local_log_dir(&runtime_config)?, "model-management")?;
        let platform = ScriptPlatform::current();
        let init_script_path = src_model_root.join(platform.init_task_runtime_relative_path());
        let download_script_path = src_model_root.join(platform.download_models_relative_path());
        let begin_llm_task_script_path = src_model_begin_llm_task_script_path(&src_model_root);
        let use_cpu_mode = device == HardwareType::Cpu;
        let bootstrap_paths = PipelineBootstrapPaths {
            base_model: &model_info.base_model,
            model_version: &model_info.model_version,
            log_dir: &log_dir,
            src_model_root: &src_model_root,
            begin_llm_task_script_path: &begin_llm_task_script_path,
            init_task_runtime_script_path: &init_script_path,
            download_models_script_path: &download_script_path,
        };

        validate_and_init(
            bootstrap_paths,
            model_id,
            use_cpu_mode,
            INIT_MODEL_RUNTIME_LABEL,
            |script_path, working_dir, _task_id, log_dir, log_path, script_args, label| async move {
                let mut args = platform.shell_args(&script_path);
                args.push("--log-path".to_string());
                args.push(log_dir.to_string_lossy().to_string());
                args.push("--task-log-file".to_string());
                args.push(log_path.to_string_lossy().to_string());
                args.extend(script_args);
                run_logged_command(
                    Path::new(platform.shell_program()),
                    &args,
                    &working_dir,
                    label,
                    &log_path,
                    "模型管理安装阶段执行完成",
                )
                .await
            },
        )
        .await?;

        validate_and_download(
            self,
            bootstrap_paths,
            model_id,
            &model_info,
            DOWNLOAD_MODEL_ARTIFACTS_LABEL,
            |script_path, working_dir, _task_id, log_dir, log_path, script_args, label| async move {
                let mut args = platform.shell_args(&script_path);
                args.push("--log-path".to_string());
                args.push(log_dir.to_string_lossy().to_string());
                args.push("--task-log-file".to_string());
                args.push(log_path.to_string_lossy().to_string());
                args.extend(script_args);
                run_logged_command(
                    Path::new(platform.shell_program()),
                    &args,
                    &working_dir,
                    label,
                    &log_path,
                    "模型管理安装阶段执行完成",
                )
                .await
            },
            || {
                validate_model_artifact_paths(
                    &model_info.base_model,
                    &model_info.model_version,
                    &resolve_model_download_paths(&src_model_root, &model_info),
                )
            },
        )
        .await?;

        Ok(ModelMutationResult {
            model: self.get_model_info_impl(model_id).await?,
            removed_paths: Vec::new(),
            preserved_paths: Vec::new(),
        })
    }

    pub(crate) async fn get_device_type_impl(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<HardwareType> {
        let model_info = self
            .find_supported_model_variant(base_model.trim(), model_version.trim())
            .await?;
        let runtime_config = self.runtime_config()?;
        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let platform = ScriptPlatform::current();
        let ensure_torch_runtime_script_path =
            src_model_root.join(platform.ensure_torch_runtime_relative_path());
        let log_dir = ensure_child_dir(&resolve_local_log_dir(&runtime_config)?, "device-check")?;
        let log_path = log_dir.join(format!(
            "detect-device-{}-{}.log",
            model_info.base_model, model_info.model_version
        ));

        let mut args = platform.shell_args(&ensure_torch_runtime_script_path);
        args.push("--log-path".to_string());
        args.push(log_dir.to_string_lossy().to_string());
        args.push("--task-log-file".to_string());
        args.push(log_path.to_string_lossy().to_string());
        args.push("--base-model".to_string());
        args.push(model_info.base_model.clone());
        args.push("--query-device-type".to_string());

        let output = run_logged_command_with_output(
            Path::new(platform.shell_program()),
            &args,
            &src_model_root,
            "查询 Torch 运行时设备类型",
            &log_path,
            "Torch 运行时设备类型查询完成",
        )
        .await?;

        Ok(parse_detected_device_type(&output).unwrap_or(HardwareType::Cpu))
    }

    pub(crate) async fn uninstall_model_impl(&self, model_id: i64) -> Result<ModelMutationResult> {
        let row = self.find_model_info_row_by_id(model_id).await?;
        let model_info = map_model_info(row.clone())?;
        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let venv_dir = src_model_root.join(&model_info.base_model).join("venv");
        let conda_env_dir = src_model_root
            .join(&model_info.base_model)
            .join("conda_env");
        let artifacts_root = src_model_root.join("base-models");
        let shared_artifacts = self.collect_shared_artifact_names(model_id).await?;
        let mut removed_paths = Vec::new();
        let mut preserved_paths = Vec::new();

        // 针对直接在src-model/base-models/下的模型文件或目录，只有在没有其他模型依赖时才删除
        for artifact_name in &model_info.required_model_name_list {
            let artifact_path = artifacts_root.join(artifact_name);
            if !artifact_path.exists() {
                continue;
            }

            if shared_artifacts.contains(artifact_name) {
                preserved_paths.push(artifact_path.to_string_lossy().to_string());
                continue;
            }

            if artifact_path.is_dir() {
                fs::remove_dir_all(&artifact_path).await.with_context(|| {
                    format!(
                        "failed to remove model artifact directory: {}",
                        artifact_path.display()
                    )
                })?;
            } else {
                fs::remove_file(&artifact_path).await.with_context(|| {
                    format!(
                        "failed to remove model artifact file: {}",
                        artifact_path.display()
                    )
                })?;
            }

            removed_paths.push(artifact_path.to_string_lossy().to_string());
        }

        // 针对通过 git clone 或其他方式下载到模型专用目录下的文件或目录，如果没有多个模型依赖则删除整个目录
        let git_project_dir = artifacts_root.join(&model_info.base_model);
        if git_project_dir.exists() {
            if shared_artifacts.contains(&model_info.base_model) {
                preserved_paths.push(git_project_dir.to_string_lossy().to_string());
            } else {
                fs::remove_dir_all(&git_project_dir)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to remove model artifact directory: {}",
                            git_project_dir.display()
                        )
                    })?;
                removed_paths.push(git_project_dir.to_string_lossy().to_string());
            }
        }

        if remove_dir_if_exists(&venv_dir).await.with_context(|| {
            format!(
                "failed to remove model runtime directory: {}",
                venv_dir.display()
            )
        })? {
            removed_paths.push(venv_dir.to_string_lossy().to_string());
        }

        if remove_dir_if_exists(&conda_env_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to remove model runtime directory: {}",
                    conda_env_dir.display()
                )
            })?
        {
            removed_paths.push(conda_env_dir.to_string_lossy().to_string());
        }

        self.set_model_downloaded_impl(&model_info.base_model, &model_info.model_version, false)
            .await?;

        Ok(ModelMutationResult {
            model: self.get_model_info_impl(model_id).await?,
            removed_paths,
            preserved_paths,
        })
    }

    async fn find_model_info_row_by_id(&self, model_id: i64) -> Result<model_info_entity::Model> {
        model_info_entity::Entity::find_by_id(model_id)
            .filter(model_info_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到目标模型"))
    }

    async fn get_model_info_impl(&self, model_id: i64) -> Result<ModelInfo> {
        let row = self.find_model_info_row_by_id(model_id).await?;
        map_model_info(row)
    }

    async fn collect_shared_artifact_names(&self, model_id: i64) -> Result<HashSet<String>> {
        let rows = model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .filter(model_info_entity::Column::Downloaded.eq(true))
            .filter(model_info_entity::Column::Id.ne(model_id))
            .all(self.orm())
            .await?;

        let mut shared = HashSet::new();
        for row in rows {
            for artifact_name in
                parse_json_field::<Vec<String>>(&row.required_model_name_list_json)?
            {
                shared.insert(artifact_name);
            }
        }

        Ok(shared)
    }
}

fn map_model_info(row: model_info_entity::Model) -> Result<ModelInfo> {
    Ok(ModelInfo {
        id: row.id,
        base_model: row.base_model,
        model_name: row.model_name,
        model_version: row.model_version,
        download_type: row
            .download_type
            .parse()
            .unwrap_or(ModelDownloadType::HfLike),
        required_model_name_list: parse_json_field(&row.required_model_name_list_json)?,
        required_model_repo_id_list: parse_json_field(&row.required_model_repo_id_list_json)?,
        supported_feature_list: parse_json_field::<Vec<String>>(&row.supported_feature_list_json)?,
        supported_devices: parse_json_field::<Vec<HardwareType>>(&row.supported_devices)?,
        downloaded: row.downloaded,
        create_time: row.create_time,
        modify_time: row.modify_time,
    })
}

fn parse_json_field<T>(value: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(value).or_else(|first_err| {
        let normalized = value.replace(r#"\""#, r#"""#);
        serde_json::from_str(&normalized).map_err(|_| first_err.into())
    })
}

fn parse_detected_device_type(output: &str) -> Option<HardwareType> {
    output
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("DEVICE_TYPE|"))
        .and_then(|value| value.trim().parse::<HardwareType>().ok())
}

async fn remove_dir_if_exists(path: &Path) -> Result<bool> {
    if !fs::try_exists(path)
        .await
        .with_context(|| format!("failed to inspect directory: {}", path.display()))?
    {
        return Ok(false);
    }

    fs::remove_dir_all(path)
        .await
        .with_context(|| format!("failed to remove directory: {}", path.display()))?;
    Ok(true)
}
