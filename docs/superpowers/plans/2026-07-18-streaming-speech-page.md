# 流式语音生成页面 + 右侧可拖拽配置抽屉 实现计划

- 日期：2026-07-18
- 分支：v.0.12.0
- 范围：仅前端初步设计（不涉及后端；复用已有 `stream_audio_placeholder` 占位命令）

## 1. 目标与关键决策

构建一个 ChatUI 风格的「流式语音生成」页面：用户在聊天框输入文本、选择说话人，回车发起流式生成，借助已有 `StreamableAudioPlayer` 实时播放。页面配置（模型/设备/语言/说话人列表/模型参数）通过一个**右侧可拖拽抽屉**承载，默认收起、按需唤起。

基于澄清确认的决策：

| 维度 | 决策 |
|------|------|
| 说话人配置范围 | 暂只实现「语音克隆式」说话人：名称 + 参考音频 + 参考文本（按模型可选）+ 类别字段。**不接入** `speakerStore`，纯前端本地管理 |
| 类别字段 | `category: StreamingSpeakerCategory`，本期固定 `'voice-clone'`，预留 `'preset' | 'trained'` 供未来扩展 |
| 抽屉展现 | 默认收起，点击页面「配置」按钮从右侧滑入；左边缘可拖拽调宽（320–560px） |
| 页面入口 | 新增普通路由 `/streaming-speech` 并加入侧边栏（**不改动**后端共享的 `HistoryTaskType` 枚举） |
| 配置结构 | 参考 `VoiceCloneView` / `TextToSpeechView`：`PanelCard` 分区 + `BaseListbox` + `GenericTaskParamsForm`（`useUiConfigStore` 驱动） |

## 2. 数据模型（新增 `src/types/streaming.ts`）

```ts
export type StreamingSpeakerCategory = 'voice-clone' | 'preset' | 'trained';

export interface StreamingSpeakerConfig {
  id: string;                       // 前端生成（自增种子 + 计数，避免 crypto 依赖）
  name: string;                     // 说话人名称
  category: StreamingSpeakerCategory;
  baseModel: string;                // 该说话人参考音频对应的模型
  modelVersion?: string;            // 模型版本（可选）
  refAudioPath: string;             // 参考音频本地绝对路径
  refAudioName: string;             // 参考音频文件名（展示用）
  refText: string;                  // 参考文本（按模型可选）
  description?: string;             // 备注
  createTime?: string;              // 后端生成，前端只读、不赋值、不向后端提供（纯前端本地阶段为空）
  modifyTime?: string;              // 后端生成，前端只读
}

export type StreamingMessageRole = 'user' | 'assistant';
export type StreamingMessageStatus = 'streaming' | 'completed' | 'error';

export interface StreamingChatMessage {
  id: string;
  role: StreamingMessageRole;
  text: string;                     // user 文本 / assistant 摘要
  speakerId: string | null;         // 该轮选用的说话人
  speakerName?: string;
  taskId: number;                   // 流式任务 id（占位，传给 stream_audio_placeholder）
  contextId: string;                // 流式上下文 id（占位，用消息 id）
  status: StreamingMessageStatus;
  taskCreateTime?: string;          // 任务创建时间：后端执行任务前生成、响应返回时填充；前端不赋值
}

export interface StreamingSessionConfig {
  baseModel: string;
  modelVersion: string;
  device: string;
  language: AppLanguage;
  modelParams: Record<string, unknown>;
}
```

> 时间字段：统一 `createTime` / `modifyTime`（类型 `string`，项目硬规范），但**一律由后端在任务执行/实体持久化前生成**，前端只读、不主动赋值、不向后端 payload 提供--与 `VoiceCloneView`/`TextToSpeechView` 任务提交-响应模式一致（前端提交 `{baseModel, refAudioPath, refText, text, ...}` 不含时间，后端返回 `{createdAt, ...}`）。纯前端本地实体（本期说话人/消息）时间字段设为可选，本地阶段留空，UI 不展示或占位；未来接入后端时由后端返回填充。id 用模块级自增计数器（非时间）。

## 3. Store（新增 `src/stores/streamingSpeech.ts`）

`defineStore('streaming-speech', ...)`，组合式 API：

**State**
- `speakers: Ref<StreamingSpeakerConfig[]>` — 本地说话人列表
- `messages: Ref<StreamingChatMessage[]>` — 聊天消息
- `sessionConfig: Reactive<StreamingSessionConfig>` — 模型/设备/语言/参数（抽屉编辑，聊天页消费）
- `isDrawerOpen: Ref<boolean>` — 抽屉开关（也可放页面级，放 store 便于多组件协同）

**Getters**
- `speakerOptions` — `speakers` 映射为 `{label, value, description}`（聊天框下拉用）
- `selectedSpeaker`（按 id 查）

**Actions**
- `addSpeaker(payload)` / `updateSpeaker(id, patch)` / `removeSpeaker(id)`
- `openDrawer()` / `closeDrawer()` / `toggleDrawer()`
- `setSessionConfig(patch)` — 抽屉编辑基础配置
- `sendMessage(text, speakerId)`：
  1. push user message
  2. 生成 `taskId = nextTaskId()`（自增）、`contextId = assistantId`
  3. push assistant message（status `'streaming'`）
  4. **不在此 invoke**——由渲染出的 `StreamableAudioPlayer`（`mode='stream'`）在 `watch immediate` 时自动调 `startStreaming(taskId, contextId)`，符合既有组件契约
  5. 初版不追踪流式结束（播放状态由 `StreamableAudioPlayer` 自管）；`status` 保留 `streaming` 即可

> 提交后端的 payload（当前 `stream_audio_placeholder` 的 `{taskId, contextId, onEvent}`，未来真实流式命令）**不含任何时间字段**；任务创建时间由后端执行前生成，未来通过任务响应回填 `taskCreateTime`，与 `VoiceCloneView.createTask` 提交无时间、后端返回 `createdAt` 的模式一致。`addSpeaker`/`updateSpeaker` 的本地 payload 同样不含时间，纯前端本地实体的 `createTime`/`modifyTime` 留空。
- `clearMessages()`

**持久化**：初版仅内存。预留 `loadFromStorage / saveToStorage` 钩子（localStorage），本期不实现，仅在记忆/注释中标明后续点。

**参考文本必填判断**（对齐 `TextToSpeechView` 的 `DYNAMIC_REFERENCE_BASE_MODELS`）：
```ts
const REQUIRES_REF_TEXT_MODELS = new Set(['gpt_sovits_cpufast']);
const requiresRefText = (baseModel: string) => REQUIRES_REF_TEXT_MODELS.has(baseModel);
```
导出供抽屉表单复用。后续可由后端模型能力字段驱动。

## 4. 抽屉组件（新增 `src/components/streaming/StreamingConfigDrawer.vue`）

非通用组件，专用于本页。

**Props**
```ts
interface Props {
  open: boolean;
  width?: number;            // 受控宽度，默认 380
}
```
**Emits**：`close`、`update:width`。

**结构**（`Teleport` 到 body，`fixed inset-y-0 right-0`）
- 遮罩层：`bg-[#7a4a24]/18 backdrop-blur-[2px]`（与 `BaseDialog` 一致），点击触发 `close`
- 面板：`bg-[#fffdfa] border-l border-brand-100 shadow-panel`，宽度绑定 `width`
- 左边缘拖拽手柄：`absolute left-0 top-0 h-full w-1.5 cursor-col-resize`，`@pointerdown` 启动拖拽
- 头部：标题「流式语音配置」+ 关闭按钮（`XMarkIcon`）
- 内容区（`overflow-y-auto`，参考其他页面的 `PanelCard` 分区）：
  1. `PanelCard`「基础配置」：模型 / 版本 / 设备 / 语言（`BaseListbox` 网格，逻辑镜像 `VoiceCloneView` 的 `watch` 同步选项 + `modelStore.getModelsByFeature(HistoryTaskType.VoiceClone)` + `getSupportedDevices/Languages`）
  2. `PanelCard`「说话人管理」：
     - 已配置说话人卡片列表（名称、类别标签、参考音频名、参考文本摘要、编辑/删除）
     - 「新增说话人」按钮 → 打开内联表单（见 §5）
  3. `PanelCard`「模型参数」：`GenericTaskParamsForm`（`v-model=sessionConfig.modelParams`，`:task-config=uiConfigStore.getTaskConfig(baseModel, HistoryTaskType.VoiceClone)`）

**拖拽实现**
```ts
const width = ref(props.width ?? 380);
const startDrag = (e: PointerEvent) => {
  e.preventDefault();
  const startX = e.clientX;
  const startW = width.value;
  const onMove = (ev: PointerEvent) => {
    const next = Math.min(560, Math.max(320, startW - (ev.clientX - startX)));
    width.value = next; emit('update:width', next);
  };
  const onUp = () => { document.removeEventListener('pointermove', onMove); document.removeEventListener('pointerup', onUp); };
  document.addEventListener('pointermove', onMove);
  document.addEventListener('pointerup', onUp);
};
```
拖拽期间给 body 加 `cursor-col-resize` / `user-select-none`。

**过渡**：用 `<Transition>`（`translate-x-full → 0`，duration-200），遮罩 opacity 过渡。不引入 headlessui（抽屉非 Dialog 语义，但可按需用；初版用原生 Transition 足够）。

## 5. 说话人表单（新增 `src/components/streaming/StreamingSpeakerForm.vue`）

复用 `BaseDialog` 作为编辑弹窗（保持与 `SpeakersView` 编辑弹窗一致的交互）。

**Props**：`open: boolean`、`speaker?: StreamingSpeakerConfig | null`（null=新增）、`baseModelOptions`
**Emits**：`close`、`submit(payload)`

**字段**（参考 `VoiceCloneView` 参考音频/参考台词输入模式）
- 说话人名称（input，必填）
- 对应模型（`BaseListbox`，来自 `modelStore.getModelsByFeature(VoiceClone)`）—— 决定参考文本是否必填
- 参考音频：`BaseButton tone=ghost` + `@tauri-apps/plugin-dialog` `open()`（filters 复用 `MODEL_TRAINING_AUDIO_FILE_EXTENSIONS`），展示文件名/路径
- 参考文本（textarea）：当 `requiresRefText(baseModel)` 为真时标注必填并参与 `canSubmit` 校验，否则标注「可选」
- 类别：只读展示「语音克隆」（`category` 固定 `voice-clone`），以说明文字告知「未来将支持更多类别」
- 备注（可选 textarea）

**校验**：`canSubmit = name.trim() && refAudioPath && (!requiresRefText || refText.trim())`

## 6. 流式语音生成页面（新增 `src/views/StreamingSpeechView.vue`）

ChatUI 风格，整体 `flex flex-col h-[calc(100vh-...)]` 占满主内容区。

**布局**
```
PageHeader（标题「流式语音」+ 右侧「配置」按钮唤起抽屉）
└─ 聊天容器（PanelCard 或裸 div，flex-1 flex-col）
   ├─ 消息列表（flex-1 overflow-y-auto）
   │   ├─ user 消息：右对齐气泡（brand 色系）
   │   └─ assistant 消息：左对齐气泡 + StreamableAudioPlayer(mode='stream', :task-id, :context-id)
   │       └─ 空状态：引导文案「输入文本开始对话，先在配置抽屉中添加说话人」
   └─ 输入区（border-t）
       ├─ 说话人选择（BaseListbox，options = store.speakerOptions；无说话人时禁用并提示）
       └─ textarea（Enter 发送 / Shift+Enter 换行）+ 发送按钮
```

**交互**
- `canSend = text.trim() && selectedSpeakerId`；无说话人时发送禁用并 `notifyWarning('请先在配置抽屉中添加说话人')`
- 回车发送 → `store.sendMessage(text, speakerId)` → 清空输入框
- `StreamableAudioPlayer` 在 assistant 消息挂载时自动 `startStreaming`（既有 `watch immediate` 契约），用户可点播放/暂停实时收听
- 抽屉由头部「配置」按钮 + `store.isDrawerOpen` 控制

**onMounted**：`uiConfigStore.ensureLoaded()` + `modelStore.ensureLoaded()`（基础配置依赖模型列表）。

## 7. 路由与导航接入

**`src/routers/index.ts`**：在 `appRoutes` 新增（普通路由，不动 `HistoryTaskType`）：
```ts
{
  path: '/streaming-speech',
  name: 'streaming-speech',
  meta: { title: '流式语音' },
  component: StreamingSpeechView
}
```

**`src/App.vue`**：`navIcons` 增加 `streaming-speech: ChatBubbleLeftRightIcon`（`@heroicons/vue/24/outline`）。侧边栏自动通过 `appRoutes` 渲染。

## 8. 文件清单

### 新增
| 文件 | 职责 |
|------|------|
| `src/types/streaming.ts` | 流式语音类型（说话人/消息/会话配置/类别） |
| `src/stores/streamingSpeech.ts` | 本地说话人 + 消息 + 会话配置 store |
| `src/components/streaming/StreamingConfigDrawer.vue` | 右侧可拖拽抽屉（基础配置 + 说话人管理 + 模型参数） |
| `src/components/streaming/StreamingSpeakerForm.vue` | 说话人新增/编辑弹窗表单 |
| `src/views/StreamingSpeechView.vue` | ChatUI 风格流式语音生成页 |

### 修改
| 文件 | 改动 |
|------|------|
| `src/routers/index.ts` | 注册 `/streaming-speech` 路由 |
| `src/App.vue` | `navIcons` 增加流式语音图标 |

### 复用（不改动）
`StreamableAudioPlayer`、`useStreamableAudioPlayer`、`PanelCard`、`BaseButton`、`BaseListbox`、`BaseDialog`、`GenericTaskParamsForm`、`useUiConfigStore`、`useModelStore`、`useUiStore`、`stream_audio_placeholder` 命令。

## 9. 实现顺序

1. `types/streaming.ts` — 类型先立
2. `stores/streamingSpeech.ts` — store 骨架（state + actions，sendMessage 先留调用骨架）
3. `StreamingSpeakerForm.vue` — 说话人表单（可独立验证）
4. `StreamingConfigDrawer.vue` — 抽屉（拖拽 + 三段 PanelCard + 接入表单）
5. `StreamingSpeechView.vue` — 聊天页（消息列表 + 输入区 + 接入抽屉 + StreamableAudioPlayer）
6. `routers/index.ts` + `App.vue` — 路由与导航接入
7. 手动验证：侧边栏入口 → 打开抽屉配置说话人 → 聊天发送 → StreamableAudioPlayer 流式播放占位正弦波

## 10. 不在本次范围

- 后端真实流式推理（仍用 `stream_audio_placeholder` 占位正弦波）
- 说话人配置持久化（localStorage，仅预留钩子）
- 接入现有 `speakerStore` / 历史记录体系
- `HistoryTaskType` 枚举扩展（保持后端共享枚举不动）
- 前端自动化测试（与既有「前端不写自动化测试」哲学一致，手动验证）
- 流式完成状态回写 store（播放状态由 `StreamableAudioPlayer` 自管）
