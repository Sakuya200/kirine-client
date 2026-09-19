---
name: i18n-architecture
description: i18n 多语言体系：vue-i18n v11 按模块词条、UiLanguage 持久化、AppError 错误码化、en 缺词回退中文
metadata:
  type: project
---

# i18n 多语言体系

状态截至 2026-09-14 · 分支 `v0.12.2`

## 架构

- **vue-i18n v11**（`legacy: false`，`fallbackLocale: 'zh-CN'`），入口 `src/locales/index.ts`，`main.ts` 在 pinia 之后注册。
- **UI 语言** = `src/enums/uiLanguage.ts` 的 `UiLanguage`（`zh-CN`/`en-US`），与合成语音语言 `AppLanguage`（中/英/日/韩，见 [[data-flow-and-types]]）是两个独立概念，禁止复用。
- **词条**按模块拆分：`src/locales/<lang>/<module>.ts`（common/settings/tts/voiceClone/voiceDesign/training/speakers/history/streaming/modelManage/errors），命名空间 `<模块>.<区域>.<名称>`。**新增页面/组件时必须同时写 zh-CN 与 en-US 两份词条**。
- **语言持久化**：Rust `BasicConfig.language: Option<String>`（config.toml `[basic]` 段）+ 独立命令 `save_ui_language`（不经 `save_settings_config`，避免触发目录迁移）。启动时 `App.vue` onMounted 调 `initUiLanguage()` 恢复；设置页「系统设置」面板顶部切换，即改即存、无需重启。
- **枚举文本表**约定：含中文的表以 `_KEY` 后缀命名（如 `STATUS_TEXT_KEY`、`HISTORY_TASK_TYPE_TEXT_KEY`、`APP_LANGUAGE_SHORT_LABELS_KEY`），value 存 i18n key，消费处 `t(XXX_TEXT_KEY[x])`。技术术语表（`HARDWARE_TYPE_TEXT`/`ATTENTION_IMPLEMENTATION_TEXT`/`MODEL_TRAINING_ANNOTATION_FORMAT_TEXT`）值本身语言无关，保持原文直显。
- **语言原生名**（`中文（简体)`/`English`/`日本語`）不走 i18n，永远以本语言显示——这是惯例不是遗漏。

## 错误码机制（渐进式）

- Rust `src-tauri/src/error.rs`：`AppError { code: Option<String>, message, params: HashMap }`（camelCase 序列化），`AppError::coded(...).with_param(...).into_anyhow()` 沿 anyhow 链抛出，hooks 边界用 `AppError::from_anyhow`（downcast 还原）。**code 常量清单与前端 `src/locales/<lang>/errors.ts` 词条必须两侧同步**。
- 首批已错误码化（约 13 个 code）：设备不支持、模型未安装、任务句柄读取/终止信号、说话人/音频/台词等字段校验、配置状态读写。12 个受影响命令（任务创建/取消、流式、模型安装/设备、说话人增改）返回 `Result<T, AppError>`，其余命令仍 `Result<T, String>`。
- 前端 `useErrorMessage.ts`：`isCodedError` 识别 `{code,message,params}` 走 `errors.<code>` 翻译（缺 key 回退中文、再回原文）；否则维持「本地化描述：原文」。
- **后续新增 Rust 报错时优先用 AppError::coded + errors.ts 词条**，未迁移的报错原文透传（英文界面下显示中文原文属预期行为）。

## 约束

- `src-model/*/configs/params-config.json` 的配置驱动表单文案（label/description）**不在 i18n 范围**，适配器契约未动。
- defineProps 默认值不能引用 `t()`（编译期提升），一律 prop 可选 + 渲染处 `?? t(...)` 回退。
- vue-i18n 消息中字面 `{}`/`@`/`|` 需转义（`{'{'}` 等），如 `training.template.jsonlHint`。

相关：[[tech-stack-frontend]]、[[tech-stack-backend]]、[[dont-borrow-feature-config]]
