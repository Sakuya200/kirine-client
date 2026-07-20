# 流式语音流水线 Code Review 修复计划

- 日期：2026-07-18
- 分支：v.0.12.0
- 范围：针对流式语音流水线（Rust 后端 + Vue 前端）首批实现的一次 code review，记录待修复项；本期**仅登记，不在当前迭代内强制修复**，供后续排期。
- 关联：实现计划见 [2026-07-18-streaming-speech-page.md](2026-07-18-streaming-speech-page.md)；流水线主干提交 `5ec4888`→`426cc54`。
- 约束：`ResolvedStreamingPaths` 的 `base_model` / `model_version` / `sample_root` 字段及 `resolve_streaming_paths` 的 `model_version` 参数为**预留待用字段**，未来须透传脚本侧，**不得擅删**（详见记忆 `retain-future-use-fields`）。本计划所有修复项均不触碰这些字段。

## 0. 严重度分级

| 级别 | 含义 | 数量 |
|------|------|------|
| 中高 | 真实使用会直接咬人（挂起 / 僵尸会话 / 丢消息记录） | 3 |
| 中 | 体验或数据一致性受损，但有规避路径 | 2 |
| 低 | 性能 / 防御性 / 可观测性缺口，当前影响有限 | 4 |

## 1. 中高优先级

### #1 `send_streaming_message` 的 `done_rx.await` 无超时

- **位置**：[src-tauri/src/service/local/streaming.rs:259-261](../../../src-tauri/src/service/local/streaming.rs#L259-L261)
- **问题**：`done_rx.await` 无超时包裹。若脚本在产出终帧（`finished`/`error`）前卡住、或 stdout 帧丢失、或 runner 在 `drain_pending_channels` 之外路径退出导致 `done_tx` 未被发送，则 `done_rx.await` 永久挂起，`send_streaming_message` 命令永不返回。
- **影响**：前端 `invoke('send_streaming_message')` 永久转圈；该消息的 Channel 不结束；会话无法正常推进，用户只能「终止会话」。
- **修复方案**：用 `tokio::time::timeout` 包裹 `done_rx.await`：
  ```rust
  let outcome = match tokio::time::timeout(Duration::from_secs(600), done_rx).await {
      Ok(Ok(o)) => o,
      Ok(Err(_)) => bail!("流式会话进程退出，未收到完成信号"),
      Err(_) => {
          // 超时：移除 channel、下发 Error、bail
          if let Ok(mut g) = extra.message_channels.write() { g.remove(&context_id); }
          let _ = on_event.send(AudioStreamEvent::Error { message: "流式生成超时".into() });
          bail!("流式生成超时（contextId={context_id}）");
      }
  };
  ```
  超时时长可由常量或配置提供（建议单消息上限 5 分钟，按真实合成耗时调整）。
- **测试建议**：单测 mock 一个永不发送的 `done_tx`，断言超时分支返回 Err 且 channel 被移除。

### #2 首条消息双重提交产生僵尸会话

- **位置**：[src/views/StreamingSpeechView.vue:51-61](../../../src/views/StreamingSpeechView.vue#L51-L61) `send()`；[src/stores/streamingSpeech.ts:111-147](../../../src/stores/streamingSpeech.ts#L111-L147) `sendMessage` 首条建会话段
- **问题**：`send()` 调 `store.sendMessage(...)` **不 await**，且 `canSend = inputText.trim() && selectedSpeakerId`（[StreamingSpeechView.vue:24](../../../src/views/StreamingSpeechView.vue#L24)）**不含 `isStartingSession` 守卫**。用户在首条消息建会话期间（`isStartingSession=true`，await `create_streaming_speech_task` 尚未返回）快速连点发送或连按回车，会发起多次 `create_streaming_speech_task`，每次都在后端建一个会话 + 拉起一个长期 runner 进程，但只有第一个返回的 taskId 会回填 `activeTaskId`，其余成为无人管理的僵尸会话。`isStartingSession` 已在 store 声明（[streamingSpeech.ts:38](../../../src/stores/streamingSpeech.ts#L38)）但**当前从未作为守卫被读取**。
- **影响**：僵尸 runner 进程占用 GPU/CPU；DB 残留多行 Running 任务，需下次启动 `sweep_stale_streaming_sessions` 才清理；用户重复发送会触发多次后端建会话。
- **修复方案**（三选一或组合）：
  1. `canSend` 增加 `!store.isStartingSession` 条件，建会话期间禁用发送按钮。
  2. `send()` 入口对 `store.isStartingSession` 提前 return（防回车连击）。
  3. `sendMessage` 内部用 `isStartingSession` 做防重入：进入时若已 true 则直接 return。
- **与 #9 的关系**：#9（`send()` 不 await 的未处理 rejection）同根，建议一并修：`send()` 改 `async`，await `sendMessage` 并 try/catch `notifyError`，同时引入守卫。

### #3 `context.json` 读-改-写并发竞争

- **位置**：[src-tauri/src/service/local/streaming.rs:223-249](../../../src-tauri/src/service/local/streaming.rs#L223-L249) `send_streaming_message_impl` 内 context.json 的 read→push→write 段
- **问题**：context.json 采用 read-modify-write，无文件锁、无 session 级串行化。同一 `task_id` 并发两条 `send_streaming_message` 时，两个请求各读一份 `messages`、各自 push 一条、各自写回——后写覆盖前写，丢失先到达的那条消息记录。`input.jsonl` 是 append-only 不受影响，但 `context.json.messages` 会缺条目。
- **影响**：`context.json` 的消息历史不完整；若脚本侧依赖 `context.json.messages` 做上下文回放，会读到缺失历史。
- **修复方案**（二选一）：
  1. 在 `StreamingSessionExtra` 增加 `context_lock: Arc<tokio::sync::Mutex<()>>`，RMW 段加锁串行化（最小改动）。
  2. 把 `messages` 追加改为 append-only 的 `messages.jsonl`（与 `input.jsonl` 一致），`context.json` 只承载 `basic`，消除 RMW 竞争（更彻底，但脚本侧读取需同步调整）。
- **注意**：方案 2 涉及脚本侧 `streaming.py` 读取约定变更，需与脚本侧改造（Task 8 阻塞项）协同。

## 2. 中优先级

### #4 assistant 消息状态永不流转到 `completed`

- **位置**：[src/components/common/StreamableAudioPlayer.vue:28-39](../../../src/components/common/StreamableAudioPlayer.vue#L28-L39) 回调仅 notify uiStore；[src/stores/streamingSpeech.ts:182-193](../../../src/stores/streamingSpeech.ts#L182-L193) `terminateSession`
- **问题**：`StreamableAudioPlayer` 的 `onStreamError` / `onPlaybackEnded` 只调 `uiStore.notify*`，**不回调 store 更新对应 message 的 `status`**；`streamComplete` 仅存在于 `useStreamableAudioPlayer` 内部（[useStreamableAudioPlayer.ts:171-174](../../../src/hooks/useStreamableAudioPlayer.ts#L171-L174)），未透传到 store。结果 assistant 消息永远停在 `streaming`。`terminateSession` 把所有 `status === 'streaming'` 的消息一律改成 `error`——由于没有 `completed` 流转，已完成播放的消息仍是 `streaming`，被误标为 `error`。
- **影响**：assistant 消息状态长期显示「流式生成中…」；终止会话后已完成的消息被错误标红。
- **修复方案**：
  1. `useStreamableAudioPlayer` 的 `onStreamFinished` / `onStreamError` 回调透传 `messageId`（或由 `StreamableAudioPlayer` 接收 `messageId` prop 并在 finished/error 时 emit）。
  2. store 据回调把对应 message 置 `completed` / `error`。
  3. `terminateSession` 保持「仅 `streaming`→`error`」逻辑（根因消除后已完成消息已是 `completed`，不会被误标）。
- **关联**：原页面实现计划 §3 第 5 点明确「初版不追踪流式结束」，本项即补齐该追踪。

### #5 runner 启动失败仅记日志，任务卡在 Pending/Running

- **位置**：[src-tauri/src/service/local/streaming.rs:158-160](../../../src-tauri/src/service/local/streaming.rs#L158-L160) `create_streaming_speech_task_impl` 内 `start_streaming_session` 失败处理；runner 入口 [src-tauri/src/service/pipeline/streaming.rs:298-398](../../../src-tauri/src/service/pipeline/streaming.rs#L298-L398) `run_streaming_session`
- **问题**：`create_streaming_speech_task_impl` 调 `start_streaming_session` 失败时仅 `error!` 记日志，**不回滚、不置 Failed、不返回 Err**，仍向前端返回 `status: Pending` 的成功响应。若 runner 在 `update_task_status_impl(Running)` 之前/之中崩掉，或 `run_streaming_session` 因 panic 提前返回未跑到最终状态更新，任务会停在 `Pending` 或 `Running`。前端建会话看似成功，后续 `send_streaming_message` 因 `status != Running` bail，但用户对根因无感知。
- **影响**：DB 残留 Pending/Running 僵尸行；前端误以为会话已建立。
- **修复方案**：`start_streaming_session` 失败时，在 `create_streaming_speech_task_impl` 内捕获后把任务置 `Failed` 并返回 `Err`，让前端 `invoke` 抛错并 `notifyError`：
  ```rust
  if let Err(err) = self.start_streaming_session(base_model.clone(), task_id) {
      error!(error = %err, task_id, "failed to start streaming session");
      let _ = self.update_task_status_impl(UpdateTaskStatusPayload {
          task_id, status: TaskStatus::Failed, duration_seconds: None,
      }).await;
      bail!("流式会话启动失败: {err}");
  }
  ```
- **可选增强**：`run_streaming_session` 用 `catch_unwind` 或确保所有早退路径都走最终状态更新（当前 `bail!` 在 spawn 任务内，外层 `start_streaming_session` 已 unregister control，但 DB status 仍可能停在 Running——需复核）。

## 3. 低优先级

### #6 每个 Chunk 取一次写锁

- **位置**：[src-tauri/src/service/pipeline/streaming.rs:491-494](../../../src-tauri/src/service/pipeline/streaming.rs#L491-L494) `forward_event` 非终帧分支
- **问题**：非终帧（`Chunk`/`Started`）分支每次 `extra.message_channels.write()` 取写锁再 `send`。`Channel::send` 取 `&self`，本可用读锁；Chunk 频率高时写锁竞争不必要。当前单 runner 单会话影响有限。
- **修复方案**：非终帧改用 `read()` + `get`（`Channel::send` 是 `&self`），仅终帧用 `write()` + `remove`：
  ```rust
  if is_terminal {
      let mut guard = extra.message_channels.write()?;
      if let Some(mc) = guard.remove(context_id) { /* send + done */ }
  } else {
      let guard = extra.message_channels.read()?;
      if let Some(mc) = guard.get(context_id) { let _ = mc.on_event.send(event); }
  }
  ```
- **优先级说明**：单会话场景下收益小，可在引入多会话/高吞吐时再改。

### #7 `context_id` 未清洗直接 join

- **位置**：[src-tauri/src/common/task_paths.rs:99-101](../../../src-tauri/src/common/task_paths.rs#L99-L101) `streaming_message_audio_path`
- **问题**：`audio_dir.join(format!("{}.wav", context_id))`，`context_id` 来自前端（当前固定 `msg-N`），未做路径段清洗。若未来 `context_id` 来源变化（如包含 `../`、控制字符），存在路径穿越风险。当前安全，属纵深防御缺口。
- **修复方案**：复用既有 `sanitize_path_segment`（[src-tauri/src/service/local/mod.rs](../../../src-tauri/src/service/local/mod.rs)）对 `context_id` 清洗后再 join。
- **优先级说明**：防御性，当前无实际漏洞。

### #8 `let _ =` 吞掉 `message_count` 自增错误

- **位置**：[src-tauri/src/service/local/streaming.rs:265-272](../../../src-tauri/src/service/local/streaming.rs#L265-L272)
- **问题**：`message_count` 自增用 `let _ =` 忽略 DB 错误，失败时静默，计数器与实际消息数不一致且无告警。
- **修复方案**：失败时 `warn!` 记日志（不必 bail，避免影响主流程）：
  ```rust
  if let Err(e) = streaming_task_entity::Entity::update_many()...exec(self.orm()).await {
      warn!(error = %e, task_id, "failed to increment message_count");
  }
  ```

### #9 `send()` 不 await 导致的未处理 rejection

- **位置**：[src/views/StreamingSpeechView.vue:58](../../../src/views/StreamingSpeechView.vue#L58)
- **问题**：`store.sendMessage(...)` 返回 Promise 但未 await 也未 `.catch`。若 `sendMessage` 内 `invoke` 抛错（如 `create_streaming_speech_task` 失败），成为未处理 rejection，用户无 UI 反馈。
- **修复方案**：`send()` 改 `async`，await `sendMessage` 并 try/catch `notifyError`；或在 `sendMessage` 内部 catch 并 notify。与 #2 同根，建议一并修。

## 4. 已确认的非问题（review 优先级误报）

code-review-graph `detect_changes` 给出的三条 review 优先级经源码核实均为**正确的既有代码，非缺陷**，无需修改，仅此备案：

| 优先级项 | 位置 | 核实结论 |
|----------|------|----------|
| `sanitize_path_segment` | [src-tauri/src/service/local/mod.rs](../../../src-tauri/src/service/local/mod.rs) | 既有实现正确（过滤非法字符 + 控制字符、trim 尾部空格/点、默认 "speaker"），且不在 `contextId` 热路径上 |
| `parse_task_status` | [src-tauri/src/service/local/history.rs:625-642](../../../src-tauri/src/service/local/history.rs#L625-L642) | trivial 的 parse 包装，无问题 |
| `nextMessageId` | [src/stores/streamingSpeech.ts:31](../../../src/stores/streamingSpeech.ts#L31) | 模块级自增种子，非时间依赖，正确 |

## 5. 建议修复顺序

1. **#2 + #9**（同根，前端防重入 + 错误反馈）——改动小、收益直接（消除僵尸会话）。
2. **#1**（done_rx 超时）——防止单消息挂起拖垮会话。
3. **#3**（context.json 并发）——数据一致性，方案 1 最小改动。
4. **#4**（assistant 状态流转）——体验改善，需打通 player→store 回调。
5. **#5**（runner 启动失败置 Failed）——可观测性 + 资源回收。
6. **#7 / #8**（防御性 + 可观测性）——顺手改。
7. **#6**（写锁优化）——多会话/高吞吐时再改。

## 6. 不在本计划范围

- 脚本侧 `streaming.py` 与 `begin_llm_task` stdout 透传改造（Task 8 阻塞项，独立排期）。
- 真实流式推理对接（当前仍走占位/半成品路径）。
- 前端自动化测试（遵循既有「前端不写自动化测试」哲学，手动验证）。
- 删除 `ResolvedStreamingPaths` 预留字段（被记忆 `retain-future-use-fields` 约束禁止）。

## 7. 验证清单（修复后回归）

- [ ] 首条消息建会话期间连点发送：仅建一个会话（#2）
- [ ] 建会话失败：前端 notifyError，DB 任务为 Failed（#5、#9）
- [ ] 脚本卡住不发终帧：单消息超时返回 Error，会话可继续（#1）
- [ ] 同一会话并发两条消息：context.json.messages 含两条（#3）
- [ ] 流式播放完成：assistant 消息置 completed（#4）
- [ ] 终止会话：仅未完成消息置 error，已完成消息保持 completed（#4）
- [ ] message_count 自增失败：日志有 warn（#8）
- [ ] context_id 含异常字符：被清洗，无路径穿越（#7）
