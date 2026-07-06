-- SQLite schema for kirine-client local database
-- Source of truth: src-tauri/src/migration/*.rs (final state after all migrations)
PRAGMA foreign_keys = ON;

CREATE TABLE
    IF NOT EXISTS app_meta (
        key TEXT NOT NULL PRIMARY KEY,
        value TEXT NOT NULL
    );

CREATE TABLE
    IF NOT EXISTS speakers (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        speaker_name TEXT NOT NULL,
        samples INTEGER NOT NULL DEFAULT 0,
        base_model TEXT NOT NULL,
        description TEXT NOT NULL DEFAULT '',
        status TEXT NOT NULL,
        source TEXT NOT NULL,
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0
    );

CREATE TABLE
    IF NOT EXISTS task_history (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        task_type TEXT NOT NULL,
        title TEXT NOT NULL,
        speaker_id INTEGER,
        speaker_name_snapshot TEXT NOT NULL,
        status TEXT NOT NULL,
        duration_seconds INTEGER NOT NULL DEFAULT 0,
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        finished_time TEXT,
        device TEXT NOT NULL DEFAULT 'cpu',
        deleted INTEGER NOT NULL DEFAULT 0
    );

CREATE TABLE
    IF NOT EXISTS model_info (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        base_model TEXT NOT NULL,
        model_name TEXT NOT NULL,
        model_version TEXT NOT NULL,
        download_type TEXT NOT NULL DEFAULT 'HF-Like',
        required_model_name_list_json TEXT NOT NULL,
        required_model_repo_id_list_json TEXT NOT NULL,
        supported_feature_list_json TEXT NOT NULL,
        supported_devices TEXT NOT NULL DEFAULT '[]',
        supported_languages TEXT NOT NULL DEFAULT '["chinese","english","japanese"]',
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        downloaded BOOLEAN NOT NULL DEFAULT false,
        deleted INTEGER NOT NULL DEFAULT 0
    );

CREATE TABLE
    IF NOT EXISTS tts_tasks (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        history_id INTEGER NOT NULL,
        speaker_id INTEGER,
        model_path TEXT,
        base_model TEXT NOT NULL,
        model_version TEXT NOT NULL,
        language TEXT NOT NULL,
        format TEXT NOT NULL,
        export_audio_name TEXT NOT NULL,
        text TEXT NOT NULL,
        model_params_json TEXT NOT NULL DEFAULT '{}',
        char_count INTEGER NOT NULL,
        file_name TEXT NOT NULL,
        output_file_path TEXT,
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0,
        CONSTRAINT fk_tts_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE,
        CONSTRAINT fk_tts_tasks_speaker FOREIGN KEY (speaker_id) REFERENCES speakers (id)
    );

CREATE TABLE
    IF NOT EXISTS model_training_tasks (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        history_id INTEGER NOT NULL,
        language TEXT NOT NULL,
        base_model TEXT NOT NULL,
        model_version TEXT NOT NULL,
        speaker_name TEXT NOT NULL,
        model_params_json TEXT NOT NULL DEFAULT '{}',
        sample_count INTEGER NOT NULL,
        samples_json TEXT NOT NULL DEFAULT '[]',
        notes_json TEXT NOT NULL,
        output_speaker_id INTEGER,
        description TEXT NOT NULL DEFAULT '',
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0,
        CONSTRAINT fk_model_training_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE,
        CONSTRAINT fk_model_training_tasks_speaker FOREIGN KEY (output_speaker_id) REFERENCES speakers (id)
    );

CREATE TABLE
    IF NOT EXISTS voice_clone_tasks (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        history_id INTEGER NOT NULL,
        base_model TEXT NOT NULL,
        model_version TEXT NOT NULL,
        language TEXT NOT NULL,
        format TEXT NOT NULL DEFAULT 'wav',
        export_audio_name TEXT NOT NULL,
        ref_audio_name TEXT NOT NULL,
        ref_audio_path TEXT NOT NULL,
        ref_text TEXT NOT NULL,
        text TEXT NOT NULL,
        model_params_json TEXT NOT NULL DEFAULT '{}',
        char_count INTEGER NOT NULL,
        file_name TEXT NOT NULL,
        output_file_path TEXT,
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0,
        CONSTRAINT fk_voice_clone_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE
    );

CREATE TABLE
    IF NOT EXISTS voice_design_tasks (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        history_id INTEGER NOT NULL,
        base_model TEXT NOT NULL,
        model_version TEXT NOT NULL,
        language TEXT NOT NULL,
        format TEXT NOT NULL DEFAULT 'wav',
        export_audio_name TEXT NOT NULL,
        prompt TEXT NOT NULL,
        text TEXT NOT NULL,
        model_params_json TEXT NOT NULL DEFAULT '{}',
        char_count INTEGER NOT NULL,
        file_name TEXT NOT NULL,
        output_file_path TEXT,
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0,
        CONSTRAINT fk_voice_design_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_info_base_model ON model_info (base_model, model_version);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tts_tasks_history_id ON tts_tasks (history_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_training_tasks_history_id ON model_training_tasks (history_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_voice_clone_tasks_history_id ON voice_clone_tasks (history_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_voice_design_tasks_history_id ON voice_design_tasks (history_id);

CREATE INDEX IF NOT EXISTS idx_task_history_type_status_time ON task_history (task_type, status, create_time);

CREATE INDEX IF NOT EXISTS idx_task_history_speaker ON task_history (speaker_id);

CREATE INDEX IF NOT EXISTS idx_speakers_status_modify_time ON speakers (status, modify_time);