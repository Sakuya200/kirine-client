-- PostgreSQL schema for kirine-client (mirror of db/tables.sql)
-- Source of truth: src-tauri/src/migration/*.rs (final state after all migrations)
-- NOTE: 该文件是 SQLite 版本 (tables.sql) 的 PostgreSQL 镜像，需与之保持同步。
--       之后一旦数据库表结构发生变更，必须同时更新 db/tables.sql 与 db/tables_pgsql.sql。

create table if not exists app_meta
(
    key   varchar(20)  not null
        primary key,
    value varchar(512) not null
);

create table if not exists speakers
(
    id           bigserial
        primary key,
    speaker_name varchar(20)               not null,
    samples      bigint   default 0        not null,
    base_model   varchar(100)              not null,
    description  text     default ''::text not null,
    status       varchar(20)               not null,
    source       varchar(20)               not null,
    create_time  timestamp                 not null,
    modify_time  timestamp                 not null,
    deleted      smallint default 0        not null
);

create table if not exists task_history
(
    id                    bigserial
        primary key,
    task_type             varchar(20)                                  not null,
    title                 varchar(100)                                 not null,
    speaker_id            bigint,
    speaker_name_snapshot varchar(20)                                  not null,
    status                varchar(20)                                  not null,
    duration_seconds      bigint      default 0                        not null,
    create_time           timestamp                                    not null,
    modify_time           timestamp                                    not null,
    finished_time         timestamp,
    device                varchar(20) default 'cpu'::character varying not null,
    deleted               smallint    default 0                        not null
);

create table if not exists model_info
(
    id                               bigserial
        primary key,
    base_model                       varchar(100)                                     not null,
    model_name                       varchar(100)                                     not null,
    model_version                    varchar(100)                                     not null,
    download_type                    varchar(20) default 'HF-Like'::character varying not null,
    required_model_name_list_json    text                                             not null,
    required_model_repo_id_list_json text                                             not null,
    supported_feature_list_json      text                                             not null,
    supported_devices                text        default '[]'::text                   not null,
    supported_languages              text        default '["chinese","english","japanese"]'::text not null,
    create_time                      timestamp                                        not null,
    modify_time                      timestamp                                        not null,
    downloaded                       boolean     default false                        not null,
    deleted                          smallint    default 0                            not null
);

create table if not exists tts_tasks
(
    id                bigserial    primary key,
    history_id        bigint                                                not null,
    speaker_id        bigint,
    model_path        text,
    base_model        varchar(100)                                          not null,
    model_version     varchar(100)                                          not null,
    language          varchar(20)                                           not null,
    format            varchar(10)                                           not null,
    export_audio_name varchar(100)                                          not null,
    text              text                                                  not null,
    model_params_json text     default '{}'::text                           not null,
    char_count        integer                                               not null,
    file_name         varchar(100)                                          not null,
    output_file_path  text,
    create_time       timestamp                                             not null,
    modify_time       timestamp                                             not null,
    deleted           smallint default 0                                    not null
);

create table if not exists model_training_tasks
(
    id                bigserial
        primary key,
    history_id        bigint                      not null,
    language          varchar(20)                 not null,
    base_model        varchar(100)                not null,
    model_version     varchar(100)                not null,
    speaker_name      varchar(20)                 not null,
    model_params_json text     default '{}'::text not null,
    sample_count      bigint                      not null,
    samples_json      text     default '[]'::text not null,
    notes_json        text                        not null,
    output_speaker_id bigint,
    description       text     default ''::text   not null,
    create_time       timestamp                   not null,
    modify_time       timestamp                   not null,
    deleted           smallint default 0          not null
);

create table if not exists voice_clone_tasks
(
    id                bigserial
        primary key,
    history_id        bigint                                       not null,
    base_model        varchar(100)                                 not null,
    model_version     varchar(100)                                 not null,
    language          varchar(20)                                  not null,
    format            varchar(10) default 'wav'::character varying not null,
    export_audio_name varchar(100)                                 not null,
    ref_audio_name    varchar(100)                                 not null,
    ref_audio_path    text                                         not null,
    ref_text          text                                         not null,
    text              text                                         not null,
    model_params_json text        default '{}'::text               not null,
    char_count        integer                                      not null,
    file_name         varchar(100)                                 not null,
    output_file_path  text,
    create_time       timestamp                                    not null,
    modify_time       timestamp                                    not null,
    deleted           smallint    default 0                        not null
);

create table if not exists voice_design_tasks
(
    id                bigserial
        primary key,
    history_id        bigint                                       not null,
    base_model        varchar(100)                                 not null,
    model_version     varchar(100)                                 not null,
    language          varchar(20)                                  not null,
    format            varchar(10) default 'wav'::character varying not null,
    export_audio_name varchar(100)                                 not null,
    prompt            text                                         not null,
    text              text                                         not null,
    model_params_json text        default '{}'::text               not null,
    char_count        integer                                      not null,
    file_name         varchar(100)                                 not null,
    output_file_path  text,
    create_time       timestamp                                    not null,
    modify_time       timestamp                                    not null,
    deleted           smallint    default 0                        not null
);

create unique index if not exists idx_model_info_base_model
    on model_info (base_model, model_version);

create unique index if not exists idx_tts_tasks_history_id
    on tts_tasks (history_id);

create unique index if not exists idx_model_training_tasks_history_id
    on model_training_tasks (history_id);

create unique index if not exists idx_voice_clone_tasks_history_id
    on voice_clone_tasks (history_id);

create unique index if not exists idx_voice_design_tasks_history_id
    on voice_design_tasks (history_id);

create index if not exists idx_task_history_type_status_time
    on task_history (task_type, status, create_time);

create index if not exists idx_task_history_speaker
    on task_history (speaker_id);

create index if not exists idx_speakers_status_modify_time
    on speakers (status, modify_time);

-- sea-orm 迁移记录表（基础设施，非应用 schema；tables.sql 中省略）
create table if not exists seaql_migrations
(
    version    varchar(100) not null
        primary key,
    applied_at bigint       not null
);
