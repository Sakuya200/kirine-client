use std::{io, path::Path};

use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};

use crate::{
    common::{
        local_paths::{
            ensure_child_dir, resolve_runtime_model_path, serialize_runtime_model_path,
            serialize_task_path,
        },
        task_paths::ensure_task_sample_dir,
    },
    service::{
        local::entity::{
            speaker as speaker_entity, task_history as task_history_entity,
            tts_task as tts_task_entity,
        },
        models::{
            CreateTextToSpeechTaskPayload, HistoryTaskType, Qwen3TtsTextToSpeechModelParams,
            SpeakerStatus, TaskStatus, TextToSpeechTaskResult, VoxCpm2TextToSpeechModelParams,
        },
        pipeline::{
            qwen3_tts::qwen3_tts_preset_custom_voice_model_path,
            resolve_inference_model_path,
            script_paths::resolve_src_model_root,
            vox_cpm2::{vox_cpm2_base_model_path, VOX_CPM2_BASE_MODEL},
        },
        LocalService,
    },
    utils::time::now_string,
    Result,
};

fn is_preset_model_root_path(src_model_root: &Path, model_path: &Path) -> bool {
    model_path.starts_with(src_model_root.join("base-models"))
}

impl LocalService {
    pub(crate) async fn create_text_to_speech_task_impl(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult> {
        let txn = self.orm().begin().await?;
        let create_time = now_string()?;
        let base_model = payload.base_model.trim().to_string();
        let speaker_id = payload.speaker_id;
        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let model_scale = payload.model_scale.trim().to_string();
        let (task_speaker_id, history_speaker_id, speaker_label, inference_model_path) =
            if base_model == VOX_CPM2_BASE_MODEL {
                let base_model_path = vox_cpm2_base_model_path(&src_model_root, &model_scale)?;
                let inference_model_path =
                    resolve_inference_model_path(&base_model, &base_model_path)?;
                (
                    0_i64,
                    None,
                    "VoxCPM2 Base Voice".to_string(),
                    inference_model_path,
                )
            } else {
                let speaker = speaker_entity::Entity::find_by_id(speaker_id)
                    .filter(speaker_entity::Column::Deleted.eq(0))
                    .filter(speaker_entity::Column::Status.eq(SpeakerStatus::Ready.as_str()))
                    .filter(speaker_entity::Column::BaseModel.eq(base_model.as_str()))
                    .one(&txn)
                    .await?;
                let speaker_label = speaker
                    .as_ref()
                    .map(|model| model.name.clone())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::NotFound,
                            "未找到与当前基础模型匹配的可用说话人",
                        )
                    })?;
                let model_root_path = speaker
                    .and_then(|model| model.model_path)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "当前说话人尚未生成可用于文本转语音的本地模型",
                        )
                    })?;
                let resolved_model_root_path = resolve_runtime_model_path(
                    Path::new(self.model_dir()),
                    &src_model_root,
                    &model_root_path,
                )?;
                let inference_model_path =
                    if is_preset_model_root_path(&src_model_root, &resolved_model_root_path) {
                        qwen3_tts_preset_custom_voice_model_path(&src_model_root, &model_scale)?
                    } else {
                        resolve_inference_model_path(&base_model, &resolved_model_root_path)?
                    };
                (
                    speaker_id,
                    Some(speaker_id),
                    speaker_label,
                    inference_model_path,
                )
            };
        let text = payload.text.trim().to_string();
        let export_audio_name = super::sanitize_file_stem(&payload.export_audio_name, "kirine_tts");
        let model_params = if base_model == VOX_CPM2_BASE_MODEL {
            serde_json::to_value(serde_json::from_value::<VoxCpm2TextToSpeechModelParams>(
                payload.model_params.clone(),
            )?)?
        } else {
            serde_json::to_value(serde_json::from_value::<Qwen3TtsTextToSpeechModelParams>(
                payload.model_params.clone(),
            )?)?
        };
        let char_count = text.chars().count();
        let title = super::build_task_title("文本转语音", Some(&speaker_label), &create_time);
        let output_dir = ensure_child_dir(Path::new(self.data_dir()), "generated")?;

        let task_history = task_history_entity::ActiveModel {
            id: NotSet,
            task_type: Set(HistoryTaskType::TextToSpeech.as_str().to_string()),
            title: Set(title),
            speaker_id: Set(history_speaker_id),
            speaker_name_snapshot: Set(speaker_label.clone()),
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
        ensure_task_sample_dir(
            Path::new(self.data_dir()),
            HistoryTaskType::TextToSpeech,
            task_id,
        )?;
        let file_name = format!(
            "{}_{}.{}",
            export_audio_name,
            task_id,
            payload.format.as_str()
        );
        let output_path = output_dir.join(&file_name);
        let serialized_output_path = serialize_task_path(Path::new(self.data_dir()), &output_path);
        let serialized_model_path = serialize_runtime_model_path(
            Path::new(self.model_dir()),
            &src_model_root,
            &inference_model_path,
        );

        tts_task_entity::Entity::insert(tts_task_entity::ActiveModel {
            id: NotSet,
            history_id: Set(task_id),
            speaker_id: Set(task_speaker_id),
            model_path: Set(Some(serialized_model_path)),
            base_model: Set(base_model.clone()),
            model_scale: Set(model_scale.clone()),
            language: Set(payload.language.as_str().to_string()),
            format: Set(payload.format.as_str().to_string()),
            export_audio_name: Set(export_audio_name.clone()),
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
        self.start_tts_inference(base_model.clone(), task_id, speaker_id)?;

        Ok(TextToSpeechTaskResult {
            task_id,
            file_name,
            speaker_id: task_speaker_id,
            speaker_label,
            base_model,
            model_scale,
            language: payload.language,
            format: payload.format,
            export_audio_name,
            duration_seconds: 0,
            text,
            model_params,
            created_at: create_time,
            status: TaskStatus::Pending,
            output_file_path: serialized_output_path,
        })
    }
}
