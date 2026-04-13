use std::io;

use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};

use crate::{
    common::local_paths::resolve_task_path,
    config::{BaseModel, HardwareType},
    service::{
        local::entity::{
            task_history as task_history_entity, training_task as training_task_entity,
            tts_task as tts_task_entity, voice_clone_task as voice_clone_task_entity,
        },
        models::{
            HistoryRecord, HistoryTaskType, ModelTrainingSampleInput, ModelTrainingTaskDetail,
            TaskStatus, TextToSpeechAudioAsset, TextToSpeechFormat, TextToSpeechTaskDetail,
            UpdateTaskStatusPayload, VoiceCloneAudioAsset, VoiceCloneTaskDetail,
        },
        LocalService,
    },
    utils::{audio::content_type_for_format, time::now_string},
    Result,
};

impl LocalService {
    async fn load_history_detail(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<serde_json::Value> {
        match task_type {
            HistoryTaskType::TextToSpeech => self.load_tts_detail(history_id).await,
            HistoryTaskType::ModelTraining => self.load_model_training_detail(history_id).await,
            HistoryTaskType::VoiceClone => self.load_voice_clone_detail(history_id).await,
        }
    }

    pub(crate) async fn list_history_records_impl(&self) -> Result<Vec<HistoryRecord>> {
        let rows = task_history_entity::Entity::find()
            .filter(task_history_entity::Column::Deleted.eq(0))
            .order_by_desc(task_history_entity::Column::CreateTime)
            .order_by_desc(task_history_entity::Column::Id)
            .all(self.orm())
            .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let history_id = row.id;
            let task_type = parse_history_task_type(&row.task_type)?;
            let detail = self.load_history_detail(history_id, task_type).await?;

            records.push(HistoryRecord {
                id: history_id,
                task_type,
                title: row.title,
                speaker: row.speaker_name_snapshot,
                status: parse_task_status(&row.status)?,
                duration_seconds: row.duration_seconds,
                create_time: row.create_time,
                modify_time: row.modify_time,
                error_message: row.error_message,
                detail,
            });
        }

        Ok(records)
    }

    pub(crate) async fn delete_history_record_impl(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool> {
        let tx = self.orm().begin().await?;
        let modify_time = now_string()?;

        let history = task_history_entity::Entity::find_by_id(history_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(&tx)
            .await?;

        let Some(history) = history else {
            return Ok(false);
        };

        let mut history_active: task_history_entity::ActiveModel = history.into();
        history_active.deleted = sea_orm::ActiveValue::Set(1);
        history_active.modify_time = sea_orm::ActiveValue::Set(modify_time.clone());
        history_active.update(&tx).await?;

        match task_type {
            HistoryTaskType::TextToSpeech => {
                tts_task_entity::Entity::update_many()
                    .col_expr(tts_task_entity::Column::Deleted, Expr::value(1))
                    .col_expr(
                        tts_task_entity::Column::ModifyTime,
                        Expr::value(modify_time.clone()),
                    )
                    .filter(tts_task_entity::Column::HistoryId.eq(history_id))
                    .filter(tts_task_entity::Column::Deleted.eq(0))
                    .exec(&tx)
                    .await?;
            }
            HistoryTaskType::ModelTraining => {
                training_task_entity::Entity::update_many()
                    .col_expr(training_task_entity::Column::Deleted, Expr::value(1))
                    .col_expr(
                        training_task_entity::Column::ModifyTime,
                        Expr::value(modify_time.clone()),
                    )
                    .filter(training_task_entity::Column::HistoryId.eq(history_id))
                    .filter(training_task_entity::Column::Deleted.eq(0))
                    .exec(&tx)
                    .await?;
            }
            HistoryTaskType::VoiceClone => {
                voice_clone_task_entity::Entity::update_many()
                    .col_expr(voice_clone_task_entity::Column::Deleted, Expr::value(1))
                    .col_expr(
                        voice_clone_task_entity::Column::ModifyTime,
                        Expr::value(modify_time.clone()),
                    )
                    .filter(voice_clone_task_entity::Column::HistoryId.eq(history_id))
                    .filter(voice_clone_task_entity::Column::Deleted.eq(0))
                    .exec(&tx)
                    .await?;
            }
        }

        tx.commit().await?;
        Ok(true)
    }

    pub(crate) async fn update_task_status_impl(
        &self,
        payload: UpdateTaskStatusPayload,
    ) -> Result<HistoryRecord> {
        let modify_time = now_string()?;
        let duration_seconds = payload.duration_seconds.unwrap_or(0);
        let finished_time = if payload.status.is_finished() {
            Some(modify_time.clone())
        } else {
            None
        };
        let error_message = payload
            .error_message
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let history = task_history_entity::Entity::find_by_id(payload.task_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?;

        let Some(history) = history else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "未找到目标任务").into());
        };

        let mut active: task_history_entity::ActiveModel = history.into();
        active.status = sea_orm::ActiveValue::Set(payload.status.as_str().to_string());
        if duration_seconds > 0 {
            active.duration_seconds = sea_orm::ActiveValue::Set(duration_seconds);
        }
        active.modify_time = sea_orm::ActiveValue::Set(modify_time);
        active.finished_time = sea_orm::ActiveValue::Set(finished_time);
        active.error_message = sea_orm::ActiveValue::Set(error_message.map(ToString::to_string));
        active.update(self.orm()).await?;

        self.get_history_record_impl(payload.task_id).await
    }

    pub(crate) async fn get_history_record_impl(&self, history_id: i64) -> Result<HistoryRecord> {
        let row = task_history_entity::Entity::find_by_id(history_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到目标任务"))?;

        let task_type = parse_history_task_type(&row.task_type)?;
        let detail = self.load_history_detail(history_id, task_type).await?;

        Ok(HistoryRecord {
            id: row.id,
            task_type,
            title: row.title,
            speaker: row.speaker_name_snapshot,
            status: parse_task_status(&row.status)?,
            duration_seconds: row.duration_seconds,
            create_time: row.create_time,
            modify_time: row.modify_time,
            error_message: row.error_message,
            detail,
        })
    }

    pub(crate) async fn read_text_to_speech_audio_impl(
        &self,
        history_id: i64,
    ) -> Result<TextToSpeechAudioAsset> {
        let history = task_history_entity::Entity::find_by_id(history_id)
            .filter(
                task_history_entity::Column::TaskType.eq(HistoryTaskType::TextToSpeech.as_str()),
            )
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到目标任务"))?;

        let status = parse_task_status(&history.status)?;

        if status != TaskStatus::Completed {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "当前任务尚未完成，无法读取音频",
            )
            .into());
        }

        let row = tts_task_entity::Entity::find()
            .filter(tts_task_entity::Column::HistoryId.eq(history_id))
            .filter(tts_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到 TTS 任务文件"))?;

        let format = row
            .format
            .parse::<TextToSpeechFormat>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let output_file_path = row.output_file_path.unwrap_or_default();
        let normalized_output_file_path = output_file_path.trim();

        if normalized_output_file_path.is_empty() {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "当前任务没有可读取的音频文件").into(),
            );
        }

        let resolved_output_file_path =
            resolve_task_path(std::path::Path::new(self.data_dir()), normalized_output_file_path);

        let bytes = tokio::fs::read(&resolved_output_file_path)
            .await
            .map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("读取音频文件失败: {}", resolved_output_file_path.display()),
                )
            })?;

        Ok(TextToSpeechAudioAsset {
            task_id: history_id,
            file_name: row.file_name,
            content_type: content_type_for_format(format).to_string(),
            bytes,
        })
    }

    pub(crate) async fn read_voice_clone_audio_impl(
        &self,
        history_id: i64,
    ) -> Result<VoiceCloneAudioAsset> {
        let history = task_history_entity::Entity::find_by_id(history_id)
            .filter(task_history_entity::Column::TaskType.eq(HistoryTaskType::VoiceClone.as_str()))
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到目标任务"))?;

        let status = parse_task_status(&history.status)?;

        if status != TaskStatus::Completed {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "当前任务尚未完成，无法读取音频",
            )
            .into());
        }

        let row = voice_clone_task_entity::Entity::find()
            .filter(voice_clone_task_entity::Column::HistoryId.eq(history_id))
            .filter(voice_clone_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到声音克隆任务文件"))?;

        let output_file_path = row.output_file_path.unwrap_or_default();
        let normalized_output_file_path = output_file_path.trim();
        let format = row
            .format
            .parse::<TextToSpeechFormat>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        if normalized_output_file_path.is_empty() {
            return Err(
                io::Error::new(io::ErrorKind::NotFound, "当前任务没有可读取的音频文件").into(),
            );
        }

        let resolved_output_file_path =
            resolve_task_path(std::path::Path::new(self.data_dir()), normalized_output_file_path);

        let bytes = tokio::fs::read(&resolved_output_file_path)
            .await
            .map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("读取音频文件失败: {}", resolved_output_file_path.display()),
                )
            })?;

        Ok(VoiceCloneAudioAsset {
            task_id: history_id,
            file_name: row.file_name,
            content_type: content_type_for_format(format).to_string(),
            bytes,
        })
    }

    pub(crate) async fn load_tts_detail(&self, history_id: i64) -> Result<serde_json::Value> {
        let row = tts_task_entity::Entity::find()
            .filter(tts_task_entity::Column::HistoryId.eq(history_id))
            .filter(tts_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到 TTS 任务详情"))?;

        Ok(serde_json::to_value(TextToSpeechTaskDetail {
            speaker_id: row.speaker_id,
            base_model: row
                .base_model
                .parse::<BaseModel>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
            hardware_type: row
                .hardware_type
                .parse::<HardwareType>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
            language: row
                .language
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            format: row
                .format
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            text: row.text,
            voice_prompt: row.voice_prompt,
            char_count: row.char_count as usize,
            file_name: row.file_name,
            output_file_path: row.output_file_path.unwrap_or_default(),
        })?)
    }

    pub(crate) async fn load_model_training_detail(
        &self,
        history_id: i64,
    ) -> Result<serde_json::Value> {
        let row = training_task_entity::Entity::find()
            .filter(training_task_entity::Column::HistoryId.eq(history_id))
            .filter(training_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到模型训练任务详情"))?;

        let samples = serde_json::from_str::<Vec<ModelTrainingSampleInput>>(&row.samples_json)?;
        let notes = serde_json::from_str::<Vec<String>>(&row.notes_json)?;

        Ok(serde_json::to_value(ModelTrainingTaskDetail {
            language: row
                .language
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            base_model: row
                .base_model
                .parse::<BaseModel>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
            hardware_type: row
                .hardware_type
                .parse::<HardwareType>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
            model_name: row.model_name,
            epoch_count: row.epoch_count,
            batch_size: row.batch_size,
            sample_count: row.sample_count,
            samples,
            notes,
        })?)
    }

    pub(crate) async fn load_voice_clone_detail(
        &self,
        history_id: i64,
    ) -> Result<serde_json::Value> {
        let row = voice_clone_task_entity::Entity::find()
            .filter(voice_clone_task_entity::Column::HistoryId.eq(history_id))
            .filter(voice_clone_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到声音克隆任务详情"))?;

        Ok(serde_json::to_value(VoiceCloneTaskDetail {
            base_model: row
                .base_model
                .parse::<BaseModel>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
            hardware_type: row
                .hardware_type
                .parse::<HardwareType>()
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
            language: row
                .language
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            format: row
                .format
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            ref_audio_name: row.ref_audio_name,
            ref_audio_path: row.ref_audio_path,
            ref_text: row.ref_text,
            text: row.text,
            char_count: row.char_count as usize,
            file_name: row.file_name,
            output_file_path: row.output_file_path.unwrap_or_default(),
        })?)
    }
}

fn parse_history_task_type(value: &str) -> Result<HistoryTaskType> {
    value
        .parse::<HistoryTaskType>()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err).into())
}

fn parse_task_status(value: &str) -> Result<TaskStatus> {
    value
        .parse::<TaskStatus>()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err).into())
}
