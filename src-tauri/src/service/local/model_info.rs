use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::de::DeserializeOwned;

use crate::{
    common::local_paths::{ensure_child_dir, resolve_local_log_dir},
    config::{load_configs, HardwareType},
    service::{
        local::entity::model_info as model_info_entity,
        models::{ModelInfo, ModelMutationResult},
        pipeline::{
            qwen3_tts::{qwen3_tts_download_script_args, qwen3_tts_prepared_model_download_paths},
            script_paths::{resolve_src_model_root, src_model_venv_python_path, ScriptPlatform},
            validate_and_download, validate_and_init, validate_downloaded_paths,
            vox_cpm2::{vox_cpm2_download_script_args, vox_cpm2_prepared_model_download_paths},
            PipelineBootstrapPaths, DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
        LocalService,
    },
    utils::process::run_logged_command,
    utils::time::now_string,
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
        model_scale: &str,
    ) -> Result<bool> {
        Ok(self
            .find_model_info_row(base_model, model_scale)
            .await?
            .map(|row| row.downloaded)
            .unwrap_or(false))
    }

    pub(crate) async fn set_model_downloaded_impl(
        &self,
        base_model: &str,
        model_scale: &str,
        downloaded: bool,
    ) -> Result<()> {
        let Some(row) = self.find_model_info_row(base_model, model_scale).await? else {
            return Ok(());
        };

        let mut active_model: model_info_entity::ActiveModel = row.into();
        active_model.downloaded = Set(downloaded);
        active_model.modify_time = Set(now_string()?);
        active_model.update(self.orm()).await?;

        Ok(())
    }

    async fn find_model_info_row(
        &self,
        base_model: &str,
        model_scale: &str,
    ) -> Result<Option<model_info_entity::Model>> {
        model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .filter(model_info_entity::Column::BaseModel.eq(base_model.trim()))
            .filter(model_info_entity::Column::ModelScale.eq(model_scale.trim()))
            .one(self.orm())
            .await
            .map_err(Into::into)
    }

    pub(crate) async fn install_model_impl(&self, model_id: i64) -> Result<ModelMutationResult> {
        let row = self.find_model_info_row_by_id(model_id).await?;
        let model_info = map_model_info(row.clone())?;
        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let log_dir = ensure_child_dir(&resolve_local_log_dir()?, "model-management")?;
        let platform = ScriptPlatform::current();
        let init_script_path = src_model_root.join(platform.init_task_runtime_relative_path());
        let download_script_path = src_model_root.join(platform.download_models_relative_path());
        let venv_python_path = src_model_venv_python_path(&src_model_root, &model_info.base_model);
        let use_cpu_mode = load_configs()?.hardware_type() == HardwareType::Cpu;
        let init_log_path = log_dir.join(format!(
            "install-{}-{}-init.log",
            model_info.base_model, model_info.model_scale
        ));
        let download_log_path = log_dir.join(format!(
            "install-{}-{}-download.log",
            model_info.base_model, model_info.model_scale
        ));
        let bootstrap_paths = PipelineBootstrapPaths {
            base_model: &model_info.base_model,
            model_scale: &model_info.model_scale,
            src_model_root: &src_model_root,
            venv_python_path: &venv_python_path,
            init_task_runtime_script_path: &init_script_path,
            download_models_script_path: &download_script_path,
        };

        validate_and_init(
            bootstrap_paths,
            model_id,
            &log_dir,
            use_cpu_mode,
            INIT_MODEL_RUNTIME_LABEL,
            |script_path, working_dir, _task_id, log_dir, script_args, label| {
                let log_path = init_log_path.clone();
                async move {
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
                }
            },
        )
        .await?;

        validate_and_download(
            self,
            bootstrap_paths,
            model_id,
            &log_dir,
            resolve_model_download_script_args(
                &src_model_root,
                &model_info.base_model,
                &model_info.model_scale,
            )?,
            DOWNLOAD_MODEL_ARTIFACTS_LABEL,
            |script_path, working_dir, _task_id, log_dir, script_args, label| {
                let log_path = download_log_path.clone();
                async move {
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
                }
            },
            || {
                validate_downloaded_paths(
                    &model_info.base_model,
                    &model_info.model_scale,
                    &resolve_model_download_paths(
                        &src_model_root,
                        &model_info.base_model,
                        &model_info.model_scale,
                    )?,
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

    pub(crate) async fn uninstall_model_impl(&self, model_id: i64) -> Result<ModelMutationResult> {
        let row = self.find_model_info_row_by_id(model_id).await?;
        let model_info = map_model_info(row.clone())?;
        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let artifacts_root = src_model_root.join("base-models");
        let shared_artifacts = self.collect_shared_artifact_names(model_id).await?;
        let mut removed_paths = Vec::new();
        let mut preserved_paths = Vec::new();

        for artifact_name in &model_info.required_model_name_list {
            let artifact_path = artifacts_root.join(artifact_name);
            if shared_artifacts.contains(artifact_name) {
                preserved_paths.push(artifact_path.to_string_lossy().to_string());
                continue;
            }

            if !artifact_path.exists() {
                continue;
            }

            if artifact_path.is_dir() {
                fs::remove_dir_all(&artifact_path).with_context(|| {
                    format!(
                        "failed to remove model artifact directory: {}",
                        artifact_path.display()
                    )
                })?;
            } else {
                fs::remove_file(&artifact_path).with_context(|| {
                    format!(
                        "failed to remove model artifact file: {}",
                        artifact_path.display()
                    )
                })?;
            }

            removed_paths.push(artifact_path.to_string_lossy().to_string());
        }

        self.set_model_downloaded_impl(&model_info.base_model, &model_info.model_scale, false)
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

fn resolve_model_download_paths(
    src_model_root: &Path,
    base_model: &str,
    model_scale: &str,
) -> Result<Vec<PathBuf>> {
    match base_model.trim() {
        "qwen3_tts" => qwen3_tts_prepared_model_download_paths(src_model_root, model_scale),
        "vox_cpm2" => vox_cpm2_prepared_model_download_paths(src_model_root, model_scale),
        other => bail!("不支持的基础模型类型: {}", other),
    }
}

fn resolve_model_download_script_args(
    src_model_root: &Path,
    base_model: &str,
    model_scale: &str,
) -> Result<Vec<String>> {
    match base_model.trim() {
        "qwen3_tts" => qwen3_tts_download_script_args(src_model_root, model_scale),
        "vox_cpm2" => vox_cpm2_download_script_args(src_model_root, model_scale),
        other => bail!("不支持的基础模型类型: {}", other),
    }
}

fn map_model_info(row: model_info_entity::Model) -> Result<ModelInfo> {
    Ok(ModelInfo {
        id: row.id,
        base_model: row.base_model,
        model_name: row.model_name,
        model_scale: row.model_scale,
        required_model_name_list: parse_json_field(&row.required_model_name_list_json)?,
        required_model_repo_id_list: parse_json_field(&row.required_model_repo_id_list_json)?,
        supported_feature_list: parse_json_field::<Vec<String>>(&row.supported_feature_list_json)?,
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
