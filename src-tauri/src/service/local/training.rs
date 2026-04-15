use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use calamine::{open_workbook_auto, Reader};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, EntityTrait, TransactionTrait,
};
use serde::Serialize;
use serde_json::Value;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::{
    common::{
        local_paths::{resolve_task_path, serialize_model_path, serialize_task_path},
        task_paths::{
            task_sample_dir, training_audios_dir, training_imports_dir, training_index_jsonl_path,
            training_reference_audio_path, training_temp_extract_dir,
        },
    },
    config::{load_configs, HardwareType},
    service::{
        local::entity::{
            speaker as speaker_entity, task_history as task_history_entity,
            training_task as training_task_entity,
        },
        models::{
            CreateModelTrainingTaskPayload, HistoryTaskType, ModelTrainingFileInput,
            ModelTrainingFileKind, ModelTrainingSampleInput, ModelTrainingSampleType,
            ModelTrainingTaskResult, SpeakerSource, SpeakerStatus, TaskStatus,
        },
        pipeline::model_paths::{llm_model_display_name, speaker_model_dir},
        LocalService,
    },
    utils::time::{generate_unique_token, now_string},
    Result,
};

#[derive(Debug, Serialize)]
struct TrainingIndexEntry {
    audio: String,
    text: String,
    ref_audio: String,
}

#[derive(Debug)]
struct PreparedTrainingData {
    index_entries: Vec<TrainingIndexEntry>,
    persisted_samples: Vec<ModelTrainingSampleInput>,
    sample_root: PathBuf,
    index_jsonl_path: PathBuf,
    ref_audio_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnnotationFileFormat {
    Jsonl,
    Xlsx,
    Xls,
}

impl LocalService {
    fn reference_text_score(text: &str) -> usize {
        text.chars().filter(|ch| !ch.is_whitespace()).count()
    }

    fn reference_audio_size(path: &Path) -> u64 {
        fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
    }

    fn select_reference_audio(index_records: &[(PathBuf, String)]) -> Result<PathBuf> {
        index_records
            .iter()
            .max_by(|(audio_a, text_a), (audio_b, text_b)| {
                let text_order =
                    Self::reference_text_score(text_a).cmp(&Self::reference_text_score(text_b));
                if text_order != Ordering::Equal {
                    return text_order;
                }

                Self::reference_audio_size(audio_a).cmp(&Self::reference_audio_size(audio_b))
            })
            .map(|(audio, _)| audio.clone())
            .ok_or_else(|| anyhow::anyhow!("无法选择参考音频"))
    }

    pub(crate) async fn create_model_training_task_impl(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult> {
        let selected_training_hardware = load_configs()?.hardware_type();
        let create_time = now_string()?;
        let sample_count = payload.samples.len() as i64;
        let speaker_name = payload.model_name.trim().to_string();
        let selected_training_mode_text = format!(
            "{} / {}",
            llm_model_display_name(payload.base_model),
            match selected_training_hardware {
                HardwareType::Cuda => "CUDA",
                HardwareType::Cpu => "CPU",
            }
        );

        let languages_json = serde_json::to_string(&vec![payload.language])?;
        let txn = self.orm().begin().await?;
        let speaker = speaker_entity::ActiveModel {
            id: NotSet,
            name: Set(speaker_name.clone()),
            languages_json: Set(languages_json),
            samples: Set(0),
            base_model: Set(payload.base_model.as_str().to_string()),
            description: Set("通过模型训练任务自动创建".to_string()),
            model_path: Set(Some(String::new())),
            status: Set(SpeakerStatus::Training.as_str().to_string()),
            source: Set(SpeakerSource::Local.as_str().to_string()),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;
        let speaker_id = speaker.id;

        let task_history = task_history_entity::ActiveModel {
            id: NotSet,
            task_type: Set(HistoryTaskType::ModelTraining.as_str().to_string()),
            title: Set(super::build_task_title(
                "模型训练",
                Some(&speaker_name),
                &create_time,
            )),
            speaker_id: Set(Some(speaker_id)),
            speaker_name_snapshot: Set(speaker_name.clone()),
            status: Set(TaskStatus::Running.as_str().to_string()),
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

        let prepared = self.prepare_training_data(task_id, &payload.samples)?;
        let speaker_model_dir = speaker_model_dir(
            Path::new(self.model_dir()),
            speaker_id,
            &super::sanitize_path_segment(&speaker_name),
        );

        let mut speaker_active_model: speaker_entity::ActiveModel = speaker.into();
        speaker_active_model.model_path = Set(Some(serialize_model_path(
            Path::new(self.model_dir()),
            &speaker_model_dir,
        )));
        speaker_active_model.samples = Set(prepared.index_entries.len() as i64);
        speaker_active_model.update(&txn).await?;

        let mut notes = vec![
            format!("共导入 {} 项样本。", sample_count),
            format!(
                "训练语言 {}，批次大小 {}。",
                payload.language, payload.batch_size
            ),
            format!("训练模式: {}。", selected_training_mode_text),
            format!("整理后训练样本 {} 条。", prepared.index_entries.len()),
            format!(
                "样本目录: {}",
                serialize_task_path(Path::new(self.data_dir()), &prepared.sample_root)
            ),
            format!(
                "参考音频: {}",
                serialize_task_path(Path::new(self.data_dir()), &prepared.ref_audio_path)
            ),
            format!(
                "索引文件: {}",
                serialize_task_path(Path::new(self.data_dir()), &prepared.index_jsonl_path)
            ),
        ];
        if matches!(selected_training_hardware, HardwareType::Cpu) {
            notes.push("当前使用 CPU 训练，速度会较慢，且可能占用较高系统资源。".into());
        }
        if payload
            .samples
            .iter()
            .any(|sample| sample.secondary_file.is_some())
        {
            notes.push("部分导入项包含标注文件，已在后端统一合并到训练索引。".into());
        }
        notes.push("训练入口已触发，等待训练阶段完成。".into());

        training_task_entity::Entity::insert(training_task_entity::ActiveModel {
            id: NotSet,
            history_id: Set(task_id),
            language: Set(payload.language.as_str().to_string()),
            base_model: Set(payload.base_model.as_str().to_string()),
            model_name: Set(speaker_name.clone()),
            epoch_count: Set(payload.epoch_count),
            batch_size: Set(payload.batch_size),
            sample_count: Set(sample_count),
            samples_json: Set(serde_json::to_string(&prepared.persisted_samples)?),
            notes_json: Set(serde_json::to_string(&notes)?),
            output_speaker_id: Set(Some(speaker_id)),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        })
        .exec(&txn)
        .await?;

        txn.commit().await?;
        self.start_training(payload.base_model, task_id, speaker_id, &speaker_name)?;

        Ok(ModelTrainingTaskResult {
            task_id,
            base_model: payload.base_model,
            model_name: speaker_name,
            sample_count,
            create_time,
            status: TaskStatus::Running,
        })
    }

    fn prepare_training_data(
        &self,
        task_id: i64,
        samples: &[ModelTrainingSampleInput],
    ) -> Result<PreparedTrainingData> {
        let sample_root = task_sample_dir(
            Path::new(self.data_dir()),
            HistoryTaskType::ModelTraining,
            task_id,
        );
        if sample_root.exists() {
            fs::remove_dir_all(&sample_root).with_context(|| {
                format!(
                    "failed to reset sample directory: {}",
                    sample_root.display()
                )
            })?;
        }
        let imports_dir = training_imports_dir(&sample_root);
        let audios_dir = training_audios_dir(&sample_root);
        fs::create_dir_all(&imports_dir).with_context(|| {
            format!(
                "failed to create imports directory: {}",
                imports_dir.display()
            )
        })?;
        fs::create_dir_all(&audios_dir).with_context(|| {
            format!(
                "failed to create audios directory: {}",
                audios_dir.display()
            )
        })?;

        let mut used_names = HashSet::new();
        let mut index_records = Vec::new();
        let mut persisted_samples = Vec::with_capacity(samples.len());

        for sample in samples {
            Self::validate_sample_files(sample)?;
            let sample_import_dir = imports_dir.join(sample.id.to_string());
            fs::create_dir_all(&sample_import_dir).with_context(|| {
                format!(
                    "failed to create sample import dir: {}",
                    sample_import_dir.display()
                )
            })?;

            match sample.sample_type {
                ModelTrainingSampleType::Single => {
                    let transcript = sample
                        .transcript_preview
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| anyhow::anyhow!("单样本缺少台词文本: {}", sample.title))?;
                    let source_audio_path = resolve_task_path(
                        Path::new(self.data_dir()),
                        &sample.primary_file.file_path,
                    );
                    let persisted_audio_path = sample_import_dir
                        .join(super::build_task_audio_file_name(&source_audio_path));
                    Self::copy_input_file(&source_audio_path, &persisted_audio_path)?;
                    let audio_path = self.copy_audio_file(
                        &persisted_audio_path,
                        &audios_dir,
                        &mut used_names,
                        &sample.id.to_string(),
                    )?;
                    index_records.push((audio_path, transcript.to_string()));
                    persisted_samples.push(ModelTrainingSampleInput {
                        id: sample.id,
                        sample_type: sample.sample_type,
                        title: sample.title.clone(),
                        detail: format!(
                            "音频文件 · {}",
                            serialize_task_path(Path::new(self.data_dir()), &persisted_audio_path)
                        ),
                        transcript_preview: sample.transcript_preview.clone(),
                        primary_file: Self::with_persisted_file_path(
                            &sample.primary_file,
                            serialize_task_path(Path::new(self.data_dir()), &persisted_audio_path),
                        ),
                        secondary_file: None,
                    });
                }
                ModelTrainingSampleType::Dataset => {
                    let annotation_file = sample.secondary_file.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("样本集缺少数据标注文件: {}", sample.title)
                    })?;
                    let source_archive_path = resolve_task_path(
                        Path::new(self.data_dir()),
                        &sample.primary_file.file_path,
                    );
                    let source_annotation_path =
                        resolve_task_path(Path::new(self.data_dir()), &annotation_file.file_path);
                    let persisted_archive_path = sample_import_dir.join(
                        Self::build_dataset_file_name("archive", &source_archive_path),
                    );
                    let persisted_annotation_path = sample_import_dir.join(
                        Self::build_dataset_file_name("annotation", &source_annotation_path),
                    );
                    Self::copy_input_file(&source_archive_path, &persisted_archive_path)?;
                    Self::copy_input_file(&source_annotation_path, &persisted_annotation_path)?;
                    self.process_dataset_sample(
                        sample,
                        &persisted_archive_path,
                        &persisted_annotation_path,
                        &sample_root,
                        &audios_dir,
                        &mut used_names,
                        &mut index_records,
                    )?;
                    persisted_samples.push(ModelTrainingSampleInput {
                        id: sample.id,
                        sample_type: sample.sample_type,
                        title: sample.title.clone(),
                        detail: format!(
                            "ZIP 压缩包 + 标注文件 · {}",
                            serialize_task_path(
                                Path::new(self.data_dir()),
                                &persisted_archive_path
                            )
                        ),
                        transcript_preview: sample.transcript_preview.clone(),
                        primary_file: Self::with_persisted_file_path(
                            &sample.primary_file,
                            serialize_task_path(
                                Path::new(self.data_dir()),
                                &persisted_archive_path,
                            ),
                        ),
                        secondary_file: Some(Self::with_persisted_file_path(
                            annotation_file,
                            serialize_task_path(
                                Path::new(self.data_dir()),
                                &persisted_annotation_path,
                            ),
                        )),
                    });
                }
            }
        }

        if index_records.is_empty() {
            bail!("未整理出可用于训练的样本数据");
        }

        let selected_audio = Self::select_reference_audio(&index_records)?;
        let ref_audio_source = selected_audio.as_path();
        let ref_audio_extension = ref_audio_source
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{}", ext))
            .unwrap_or_default();
        let ref_audio_path = training_reference_audio_path(&sample_root, &ref_audio_extension);
        fs::copy(ref_audio_source, &ref_audio_path).with_context(|| {
            format!(
                "failed to copy reference audio from {} to {}",
                ref_audio_source.display(),
                ref_audio_path.display()
            )
        })?;

        let index_jsonl_path = training_index_jsonl_path(&sample_root);
        let ref_audio_string = ref_audio_path.to_string_lossy().to_string();
        let entries = index_records
            .into_iter()
            .map(|(audio, text)| TrainingIndexEntry {
                audio: audio.to_string_lossy().to_string(),
                text,
                ref_audio: ref_audio_string.clone(),
            })
            .collect::<Vec<_>>();
        let jsonl = entries
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        fs::write(&index_jsonl_path, format!("{}\n", jsonl)).with_context(|| {
            format!(
                "failed to write training index: {}",
                index_jsonl_path.display()
            )
        })?;

        Ok(PreparedTrainingData {
            index_entries: entries,
            persisted_samples,
            sample_root,
            ref_audio_path,
            index_jsonl_path,
        })
    }

    fn validate_sample_files(sample: &ModelTrainingSampleInput) -> Result<()> {
        match sample.sample_type {
            ModelTrainingSampleType::Single => {
                if sample.primary_file.file_kind != ModelTrainingFileKind::Audio {
                    bail!("单样本的主文件必须是音频文件: {}", sample.title);
                }
                if sample.secondary_file.is_some() {
                    bail!("单样本不应包含辅助文件: {}", sample.title);
                }
            }
            ModelTrainingSampleType::Dataset => {
                if sample.primary_file.file_kind != ModelTrainingFileKind::Archive {
                    bail!("样本集的主文件必须是 ZIP 压缩包: {}", sample.title);
                }

                let annotation_file = sample
                    .secondary_file
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("样本集缺少数据标注文件: {}", sample.title))?;
                if annotation_file.file_kind != ModelTrainingFileKind::Annotation {
                    bail!("样本集的辅助文件必须是标注文件: {}", sample.title);
                }
            }
        }

        Ok(())
    }

    fn process_dataset_sample(
        &self,
        sample: &ModelTrainingSampleInput,
        archive_path: &Path,
        annotation_path: &Path,
        sample_root: &Path,
        audios_dir: &Path,
        used_names: &mut HashSet<String>,
        index_records: &mut Vec<(PathBuf, String)>,
    ) -> Result<()> {
        if !archive_path.exists() {
            bail!("样本集压缩包不存在: {}", archive_path.display());
        }
        if !annotation_path.exists() {
            bail!("样本集标注文件不存在: {}", annotation_path.display());
        }
        let extension = archive_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension != "zip" {
            bail!("当前仅支持 ZIP 数据集压缩包: {}", archive_path.display());
        }

        let extract_root = training_temp_extract_dir(sample_root, sample.id);
        if extract_root.exists() {
            fs::remove_dir_all(&extract_root).with_context(|| {
                format!(
                    "failed to clean temporary extract dir: {}",
                    extract_root.display()
                )
            })?;
        }
        fs::create_dir_all(&extract_root).with_context(|| {
            format!(
                "failed to create temporary extract dir: {}",
                extract_root.display()
            )
        })?;

        self.extract_zip_archive(archive_path, &extract_root)?;
        let audio_map = self.collect_audio_candidates(&extract_root)?;
        let annotation_rows = self.read_annotation_rows(annotation_path)?;

        for (index, (audio_hint, text)) in annotation_rows.into_iter().enumerate() {
            let source_audio = audio_map.get(&audio_hint).or_else(|| {
                Path::new(&audio_hint)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| audio_map.get(&name.to_ascii_lowercase()))
            });
            let Some(source_audio) = source_audio else {
                bail!("标注文件中的音频未在压缩包内找到: {}", audio_hint);
            };

            let audio_path = self.copy_audio_file(
                source_audio,
                audios_dir,
                used_names,
                &format!("{}-{}", sample.id, index),
            )?;
            index_records.push((audio_path, text));
        }

        fs::remove_dir_all(&extract_root).with_context(|| {
            format!(
                "failed to clean extracted dataset dir: {}",
                extract_root.display()
            )
        })?;
        Ok(())
    }

    fn copy_audio_file(
        &self,
        source_path: &Path,
        audios_dir: &Path,
        used_names: &mut HashSet<String>,
        sample_id: &str,
    ) -> Result<PathBuf> {
        if !source_path.exists() {
            bail!("音频文件不存在: {}", source_path.display());
        }
        if !Self::is_audio_file(source_path) {
            bail!("不支持的音频文件类型: {}", source_path.display());
        }

        let base_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow::anyhow!("非法文件名: {}", source_path.display()))?;
        let mut candidate_name = super::build_sample_file_name(sample_id, base_name);
        while used_names.contains(&candidate_name) {
            candidate_name =
                super::build_sample_file_name(&generate_unique_token(sample_id), base_name);
        }
        used_names.insert(candidate_name.clone());

        let target_path = audios_dir.join(candidate_name);
        fs::copy(source_path, &target_path).with_context(|| {
            format!(
                "failed to copy audio from {} to {}",
                source_path.display(),
                target_path.display()
            )
        })?;
        Ok(target_path)
    }

    fn copy_input_file(source_path: &Path, target_path: &Path) -> Result<()> {
        if !source_path.exists() {
            bail!("输入文件不存在: {}", source_path.display());
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create parent dir: {}", parent.display()))?;
        }

        fs::copy(source_path, target_path).with_context(|| {
            format!(
                "failed to copy file from {} to {}",
                source_path.display(),
                target_path.display()
            )
        })?;
        Ok(())
    }

    fn with_persisted_file_path(
        file: &ModelTrainingFileInput,
        file_path: String,
    ) -> ModelTrainingFileInput {
        ModelTrainingFileInput {
            file_name: file.file_name.clone(),
            file_kind: file.file_kind,
            file_path,
        }
    }

    fn build_dataset_file_name(prefix: &str, source_path: &Path) -> String {
        let extension = source_path
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| !ext.trim().is_empty())
            .map(|ext| format!(".{}", ext))
            .unwrap_or_default();
        format!("{}{}", prefix, extension)
    }

    fn extract_zip_archive(&self, archive_path: &Path, target_dir: &Path) -> Result<()> {
        let file = File::open(archive_path)
            .with_context(|| format!("failed to open archive: {}", archive_path.display()))?;
        let mut archive = ZipArchive::new(file)
            .with_context(|| format!("failed to parse zip archive: {}", archive_path.display()))?;

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let Some(enclosed_name) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
                continue;
            };
            let output_path = target_dir.join(enclosed_name);
            if entry.is_dir() {
                fs::create_dir_all(&output_path).with_context(|| {
                    format!("failed to create extracted dir: {}", output_path.display())
                })?;
                continue;
            }

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create extracted parent dir: {}",
                        parent.display()
                    )
                })?;
            }
            let mut output = File::create(&output_path).with_context(|| {
                format!("failed to create extracted file: {}", output_path.display())
            })?;
            std::io::copy(&mut entry, &mut output).with_context(|| {
                format!(
                    "failed to extract archive entry to {}",
                    output_path.display()
                )
            })?;
        }

        Ok(())
    }

    fn collect_audio_candidates(&self, root: &Path) -> Result<HashMap<String, PathBuf>> {
        let mut audio_map = HashMap::new();
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            if !entry.file_type().is_file() || !Self::is_audio_file(entry.path()) {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/")
                .to_ascii_lowercase();
            audio_map.insert(relative, entry.path().to_path_buf());
            if let Some(file_name) = entry.path().file_name().and_then(|name| name.to_str()) {
                audio_map
                    .entry(file_name.to_ascii_lowercase())
                    .or_insert_with(|| entry.path().to_path_buf());
            }
        }

        if audio_map.is_empty() {
            bail!("压缩包中未找到可用音频文件");
        }

        Ok(audio_map)
    }

    fn read_annotation_rows(&self, annotation_path: &Path) -> Result<Vec<(String, String)>> {
        let normalized_jsonl_path = match Self::detect_annotation_format(annotation_path)? {
            AnnotationFileFormat::Jsonl => annotation_path.to_path_buf(),
            AnnotationFileFormat::Xlsx | AnnotationFileFormat::Xls => {
                let rows = Self::read_annotation_excel(annotation_path)?;
                let normalized_jsonl_path = annotation_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join("annotation.normalized.jsonl");
                Self::write_annotation_jsonl(&normalized_jsonl_path, &rows)?;
                normalized_jsonl_path
            }
        };

        self.read_annotation_jsonl(&normalized_jsonl_path)
    }

    fn read_annotation_jsonl(&self, annotation_path: &Path) -> Result<Vec<(String, String)>> {
        let file = File::open(annotation_path).with_context(|| {
            format!(
                "failed to open annotation jsonl: {}",
                annotation_path.display()
            )
        })?;
        let reader = BufReader::new(file);
        let mut rows = Vec::new();

        for (line_number, line) in reader.lines().enumerate() {
            let line = line
                .with_context(|| format!("failed to read annotation line {}", line_number + 1))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value = serde_json::from_str::<Value>(trimmed).with_context(|| {
                format!("annotation line {} is not valid json", line_number + 1)
            })?;
            let audio = Self::extract_string_field(
                &value,
                &["audio", "audio_path", "path", "file", "file_name", "name"],
            )
            .ok_or_else(|| anyhow::anyhow!("标注第 {} 行缺少音频路径字段", line_number + 1))?;
            let text = Self::extract_string_field(
                &value,
                &["text", "transcript", "sentence", "caption", "label"],
            )
            .ok_or_else(|| anyhow::anyhow!("标注第 {} 行缺少文本字段", line_number + 1))?;
            rows.push((audio.replace('\\', "/").to_ascii_lowercase(), text));
        }

        if rows.is_empty() {
            bail!("标注文件中没有可用样本: {}", annotation_path.display());
        }

        Ok(rows)
    }

    fn detect_annotation_format(annotation_path: &Path) -> Result<AnnotationFileFormat> {
        match annotation_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("jsonl") => Ok(AnnotationFileFormat::Jsonl),
            Some("xlsx") => Ok(AnnotationFileFormat::Xlsx),
            Some("xls") => Ok(AnnotationFileFormat::Xls),
            _ => bail!("暂不支持的数据标注文件类型: {}", annotation_path.display()),
        }
    }

    fn read_annotation_excel(annotation_path: &Path) -> Result<Vec<(String, String)>> {
        let mut workbook = open_workbook_auto(annotation_path).with_context(|| {
            format!(
                "failed to open annotation spreadsheet: {}",
                annotation_path.display()
            )
        })?;

        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| {
                anyhow::anyhow!("数据标注文件中没有工作表: {}", annotation_path.display())
            })?
            .with_context(|| {
                format!(
                    "failed to read first sheet from {}",
                    annotation_path.display()
                )
            })?;

        let mut rows = Vec::new();
        for (row_index, row) in range.rows().enumerate() {
            let file_name = row
                .first()
                .map(ToString::to_string)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            let text = row
                .get(1)
                .map(ToString::to_string)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());

            if row_index == 0 {
                let header_file_name = file_name
                    .as_deref()
                    .map(|value| value.to_ascii_lowercase())
                    .unwrap_or_default();
                let header_text = text
                    .as_deref()
                    .map(|value| value.to_ascii_lowercase())
                    .unwrap_or_default();
                if matches!(
                    header_file_name.as_str(),
                    "文件名" | "filename" | "file_name" | "audio" | "file"
                ) && matches!(
                    header_text.as_str(),
                    "台词" | "text" | "transcript" | "label"
                ) {
                    continue;
                }
            }

            if file_name.is_none() && text.is_none() {
                continue;
            }

            let file_name = file_name.ok_or_else(|| {
                anyhow::anyhow!("数据标注文件第 {} 行缺少文件名（第一列）", row_index + 1)
            })?;
            let text = text.ok_or_else(|| {
                anyhow::anyhow!("数据标注文件第 {} 行缺少台词（第二列）", row_index + 1)
            })?;

            rows.push((file_name.replace('\\', "/").to_ascii_lowercase(), text));
        }

        if rows.is_empty() {
            bail!("数据标注文件中没有可用样本: {}", annotation_path.display());
        }

        Ok(rows)
    }

    fn write_annotation_jsonl(annotation_path: &Path, rows: &[(String, String)]) -> Result<()> {
        let jsonl = rows
            .iter()
            .map(|(audio, text)| serde_json::json!({ "audio": audio, "text": text }).to_string())
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(annotation_path, format!("{}\n", jsonl)).with_context(|| {
            format!(
                "failed to write normalized annotation jsonl: {}",
                annotation_path.display()
            )
        })?;
        Ok(())
    }

    fn extract_string_field(value: &Value, keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|key| {
            value
                .get(*key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .map(ToOwned::to_owned)
        })
    }

    fn is_audio_file(path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase())
                .as_deref(),
            Some("wav") | Some("mp3") | Some("flac") | Some("ogg") | Some("m4a")
        )
    }
}
