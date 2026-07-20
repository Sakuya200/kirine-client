use std::path::Path;

use anyhow::bail;
use sea_orm::{
    ActiveValue::NotSet, ActiveValue::Set, EntityTrait, TransactionTrait,
    ActiveModelTrait,
};
use tokio::sync::watch;

use crate::{
    common::{
        local_paths::{ensure_child_dir, serialize_task_path},
        task_paths::ensure_task_sample_dir,
    },
    config::BaseModel,
    service::{
        local::entity::{
            task_history as task_history_entity, voice_design_task as voice_design_task_entity,
        },
        models::{
            CreateVoiceDesignTaskPayload, HistoryTaskType, TaskStatus, VoiceDesignTaskResult,
        },
        pipeline::{resolve_model_task_pipeline, VoiceDesignPipelineRequest},
        LocalService,
    },
    utils::time::now_string,
    Result,
};

impl LocalService {
    pub(crate) async fn create_voice_design_task_impl(
        &self,
        payload: CreateVoiceDesignTaskPayload,
    ) -> Result<VoiceDesignTaskResult> {
        let create_time = now_string()?;
        let base_model = payload.base_model.trim().to_string();
        let model_version = payload.model_version.trim().to_string();
        let device = payload.device;
        let selected_model_info = self
            .find_supported_model_variant(&base_model, &model_version)
            .await?;
        if !selected_model_info.supported_devices.contains(&device) {
            bail!(
                "模型 {} {} 不支持设备 {}，请切换为 {:?}",
                selected_model_info.model_name,
                selected_model_info.model_version,
                device,
                selected_model_info.supported_devices
            );
        }

        let prompt = payload.prompt.trim().to_string();
        if prompt.is_empty() {
            bail!("音色描述不能为空");
        }

        let text = payload.text.trim().to_string();
        if text.is_empty() {
            bail!("目标台词不能为空");
        }

        let export_audio_name = super::sanitize_file_stem(&payload.export_audio_name, "kirine_voice_design");
        let mut model_params = payload.model_params.clone();
        let char_count = text.chars().count();
        let title = super::build_task_title("音色设计", None, &create_time);
        let output_dir = ensure_child_dir(Path::new(self.data_dir()), "generated")?;
        let txn = self.orm().begin().await?;

        let task_history = task_history_entity::ActiveModel {
            id: NotSet,
            task_type: Set(HistoryTaskType::VoiceDesign.as_str().to_string()),
            title: Set(title),
            speaker_id: Set(None),
            speaker_name_snapshot: Set("-".to_string()),
            status: Set(TaskStatus::Pending.as_str().to_string()),
            duration_seconds: Set(0),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            finished_time: Set(None),
            device: Set(device.as_str().to_string()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;
        let task_id = task_history.id;

        let sample_dir = ensure_task_sample_dir(
            Path::new(self.data_dir()),
            HistoryTaskType::VoiceDesign,
            task_id,
        )?;

        super::copy_model_param_files(
            &base_model,
            HistoryTaskType::VoiceDesign,
            &mut model_params,
            &sample_dir,
            Path::new(self.data_dir()),
            self.ui_config(),
        )?;

        let file_name = format!("{}.{}", export_audio_name, payload.format.as_str());
        let output_path = output_dir.join(&file_name);
        let serialized_output_path = serialize_task_path(Path::new(self.data_dir()), &output_path);

        voice_design_task_entity::Entity::insert(voice_design_task_entity::ActiveModel {
            id: NotSet,
            history_id: Set(task_id),
            base_model: Set(base_model.clone()),
            model_version: Set(model_version.clone()),
            language: Set(payload.language.as_str().to_string()),
            format: Set(payload.format.as_str().to_string()),
            export_audio_name: Set(export_audio_name.clone()),
            prompt: Set(prompt.clone()),
            text: Set(text.clone()),
            model_params_json: Set(serde_json::to_string(&model_params)?),
            char_count: Set(char_count as i64),
            file_name: Set(file_name.clone()),
            output_file_path: Set(Some(serialized_output_path.clone())),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        })
        .exec(&txn)
        .await?;

        txn.commit().await?;
        self.start_voice_design_inference(base_model.clone(), task_id)?;

        Ok(VoiceDesignTaskResult {
            task_id,
            file_name,
            base_model,
            model_version,
            language: payload.language,
            format: payload.format,
            export_audio_name,
            duration_seconds: 0,
            prompt,
            text,
            model_params,
            created_at: create_time,
            status: TaskStatus::Pending,
            output_file_path: serialized_output_path,
        })
    }

    pub(crate) fn start_voice_design_inference(
        &self,
        base_model: BaseModel,
        task_id: i64,
    ) -> Result<()> {
        let service = self.clone();
        let pipeline = resolve_model_task_pipeline(&base_model)?;
        let (cancel_tx, cancel_rx_guard) = watch::channel(false);
        self.register_active_task_control(
            task_id,
            HistoryTaskType::VoiceDesign,
            cancel_tx,
            cancel_rx_guard,
        );

        tauri::async_runtime::spawn(async move {
            let result = pipeline
                .run_voice_design_pipeline(
                    base_model.to_string(),
                    &service,
                    VoiceDesignPipelineRequest { task_id },
                )
                .await;
            service.unregister_active_task_control(task_id);

            if let Err(err) = result {
                tracing::error!(error = %err, "local voice design pipeline failed");
            }
        });

        Ok(())
    }
}
