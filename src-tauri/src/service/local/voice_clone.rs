use std::{fs, path::Path};

use anyhow::{bail, Context};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, EntityTrait, TransactionTrait,
};

use crate::{
    common::{
        local_paths::{ensure_child_dir, resolve_task_path, serialize_task_path},
        task_paths::ensure_task_sample_dir,
    },
    service::{
        local::entity::{
            task_history as task_history_entity, voice_clone_task as voice_clone_task_entity,
        },
        models::{CreateVoiceCloneTaskPayload, HistoryTaskType, TaskStatus, VoiceCloneTaskResult},
        models::{VoxCpm2VoiceCloneMode, VoxCpm2VoiceCloneModelParams},
        LocalService,
    },
    utils::time::now_string,
    Result,
};

impl LocalService {
    pub(crate) async fn create_voice_clone_task_impl(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult> {
        let txn = self.orm().begin().await?;
        let create_time = now_string()?;
        let base_model = payload.base_model.trim().to_string();
        let ref_audio_path = payload.ref_audio_path.trim().to_string();
        let ref_text = payload.ref_text.trim().to_string();
        let text = payload.text.trim().to_string();

        if ref_audio_path.is_empty() {
            bail!("参考音频不能为空");
        }
        if text.is_empty() {
            bail!("目标台词不能为空");
        }
        if base_model == "vox_cpm2" {
            let params = serde_json::from_value::<VoxCpm2VoiceCloneModelParams>(
                payload.model_params.clone(),
            )?;
            if matches!(params.mode, VoxCpm2VoiceCloneMode::Ultimate) && ref_text.is_empty() {
                bail!("Ultimate 克隆模式要求填写参考音频台词");
            }
        } else if ref_text.is_empty() {
            bail!("参考音频台词不能为空");
        }
        let resolved_ref_audio_path =
            resolve_task_path(Path::new(self.data_dir()), &ref_audio_path);
        if !resolved_ref_audio_path.exists() {
            bail!("参考音频文件不存在: {}", ref_audio_path);
        }

        let ref_audio_name = if payload.ref_audio_name.trim().is_empty() {
            resolved_ref_audio_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("reference.wav")
                .to_string()
        } else {
            payload.ref_audio_name.trim().to_string()
        };
        let model_scale = payload.model_scale.trim().to_string();
        let export_audio_name =
            super::sanitize_file_stem(&payload.export_audio_name, "kirine_voice_clone");
        let char_count = text.chars().count();
        let speaker_snapshot = "-";
        let title = super::build_task_title("声音克隆", None, &create_time);
        let output_dir = ensure_child_dir(Path::new(self.data_dir()), "generated")?;

        let task_history = task_history_entity::ActiveModel {
            id: NotSet,
            task_type: Set(HistoryTaskType::VoiceClone.as_str().to_string()),
            title: Set(title),
            speaker_id: Set(None),
            speaker_name_snapshot: Set(speaker_snapshot.to_string()),
            status: Set(TaskStatus::Pending.as_str().to_string()),
            duration_seconds: Set(0),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            finished_time: Set(None),
            error_message: Set(None),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;
        let task_id = task_history.id;
        let sample_dir = ensure_task_sample_dir(
            Path::new(self.data_dir()),
            HistoryTaskType::VoiceClone,
            task_id,
        )?;
        let file_name = format!("{}.{}", export_audio_name, payload.format.as_str());
        let output_path = output_dir.join(&file_name);
        let ref_audio_target_path =
            sample_dir.join(super::build_task_audio_file_name(&resolved_ref_audio_path));
        fs::copy(&resolved_ref_audio_path, &ref_audio_target_path).with_context(|| {
            format!(
                "failed to copy reference audio from {} to {}",
                resolved_ref_audio_path.display(),
                ref_audio_target_path.display()
            )
        })?;
        let serialized_ref_audio_path =
            serialize_task_path(Path::new(self.data_dir()), &ref_audio_target_path);
        let serialized_output_path = serialize_task_path(Path::new(self.data_dir()), &output_path);

        voice_clone_task_entity::Entity::insert(voice_clone_task_entity::ActiveModel {
            id: NotSet,
            history_id: Set(task_id),
            base_model: Set(base_model.clone()),
            model_scale: Set(model_scale.clone()),
            language: Set(payload.language.as_str().to_string()),
            format: Set(payload.format.as_str().to_string()),
            export_audio_name: Set(export_audio_name.clone()),
            ref_audio_name: Set(ref_audio_name.clone()),
            ref_audio_path: Set(serialized_ref_audio_path.clone()),
            ref_text: Set(ref_text.clone()),
            text: Set(text.clone()),
            model_params_json: Set(serde_json::to_string(&payload.model_params)?),
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
        self.start_voice_clone_inference(base_model.clone(), task_id)?;

        Ok(VoiceCloneTaskResult {
            task_id,
            file_name,
            ref_audio_name,
            base_model,
            model_scale,
            language: payload.language,
            format: payload.format,
            export_audio_name,
            duration_seconds: 0,
            ref_text,
            text,
            model_params: payload.model_params,
            created_at: create_time,
            status: TaskStatus::Pending,
            output_file_path: serialized_output_path,
        })
    }
}
