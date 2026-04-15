import argparse
import importlib
from dataclasses import dataclass
import json
import os
from pathlib import Path
import shutil
import sys
from types import SimpleNamespace

target_speaker_embedding = None
DEFAULT_QLORA_TARGET_MODULES = (
    "q_proj",
    "k_proj",
    "v_proj",
    "o_proj",
    "gate_proj",
    "up_proj",
    "down_proj",
    "linear_fc1",
    "linear_fc2",
    "small_to_mtp_projection",
)


@dataclass
class TrainingRuntimeOptions:
    is_cpu: bool
    use_qlora: bool
    accelerator_kwargs: dict[str, object]
    model_load_kwargs: dict[str, object]
    qlora_support: object | None = None


@dataclass
class TrainingPipelineContext:
    runtime: TrainingRuntimeOptions
    accelerator: object
    qwen3tts: object
    model: object
    optimizer: object
    train_dataloader: object
    train_data: list[dict[str, object]]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--init_model_path",
        "--init-model-path",
        dest="init_model_path",
        type=str,
        default="Qwen/Qwen3-TTS-12Hz-1.7B-Base",
    )
    parser.add_argument(
        "--output_model_path",
        "--output-model-path",
        dest="output_model_path",
        type=str,
        default="output",
    )
    parser.add_argument("--train_jsonl", "--train-jsonl", dest="train_jsonl", type=str, required=True)
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--batch_size", "--batch-size", dest="batch_size", type=int, default=2)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--num_epochs", "--num-epochs", dest="num_epochs", type=int, default=3)
    parser.add_argument("--speaker_name", "--speaker-name", dest="speaker_name", type=str, default="speaker_test")
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument(
        "--attn_implementation",
        "--attn-implementation",
        dest="attn_implementation",
        type=str,
        default="flash_attention_2",
    )
    parser.add_argument(
        "--use-qlora",
        dest="use_qlora",
        action=argparse.BooleanOptionalAction,
        default=None,
    )
    parser.add_argument("--qlora-r", dest="qlora_r", type=int, default=16)
    parser.add_argument("--qlora-alpha", dest="qlora_alpha", type=int, default=32)
    parser.add_argument("--qlora-dropout", dest="qlora_dropout", type=str, default="0.05")
    parser.add_argument(
        "--qlora-quant-type",
        dest="qlora_quant_type",
        choices=("nf4", "fp4"),
        default="nf4",
    )
    parser.add_argument(
        "--qlora-double-quant",
        dest="qlora_double_quant",
        action=argparse.BooleanOptionalAction,
        default=True,
    )
    return parser.parse_args(argv)


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def normalize_speaker_name(speaker_name: str) -> str:
    normalized_name = speaker_name.strip().lower()
    if not normalized_name:
        raise ValueError("speaker_name must not be empty")
    return normalized_name


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


def load_qlora_support(strict: bool) -> object | None:
    try:
        importlib.import_module("bitsandbytes")
        peft_module = importlib.import_module("peft")
        transformers_module = importlib.import_module("transformers")
    except ImportError as exc:
        if strict:
            raise RuntimeError(
                "QLoRA requires the optional 'peft' and 'bitsandbytes' packages. "
                "Ensure runtime initialization completed and the CUDA environment supports bitsandbytes."
            ) from exc
        return None

    return SimpleNamespace(
        BitsAndBytesConfig=transformers_module.BitsAndBytesConfig,
        LoraConfig=peft_module.LoraConfig,
        get_peft_model=peft_module.get_peft_model,
        prepare_model_for_kbit_training=peft_module.prepare_model_for_kbit_training,
    )


def load_training_dependencies() -> SimpleNamespace:
    ensure_src_root_on_path()

    import torch
    from accelerate import Accelerator
    from qwen3_tts.dataset import TTSDataset
    from qwen_tts.inference.qwen3_tts_model import Qwen3TTSModel
    from safetensors.torch import save_file
    from torch.optim import AdamW
    from torch.utils.data import DataLoader
    from transformers import AutoConfig

    return SimpleNamespace(
        torch=torch,
        Accelerator=Accelerator,
        TTSDataset=TTSDataset,
        Qwen3TTSModel=Qwen3TTSModel,
        save_file=save_file,
        AdamW=AdamW,
        DataLoader=DataLoader,
        AutoConfig=AutoConfig,
    )


def build_runtime_options(args: argparse.Namespace, torch_module) -> TrainingRuntimeOptions:
    accelerator_kwargs: dict[str, object] = {
        "gradient_accumulation_steps": 4,
        "log_with": "tensorboard",
    }
    if args.logging_dir:
        accelerator_kwargs["project_dir"] = args.logging_dir

    if is_cpu_device(args.device):
        if args.use_qlora:
            raise ValueError("QLoRA requires a CUDA device; CPU mode is not supported.")
        return TrainingRuntimeOptions(
            is_cpu=True,
            use_qlora=False,
            accelerator_kwargs={
                **accelerator_kwargs,
                "mixed_precision": "no",
                "cpu": True,
            },
            model_load_kwargs={
                "torch_dtype": torch_module.float32,
            },
        )

    qlora_support = None
    model_load_kwargs: dict[str, object] = {
        "torch_dtype": torch_module.bfloat16,
        "attn_implementation": args.attn_implementation,
    }

    if args.use_qlora is not False:
        qlora_support = load_qlora_support(strict=args.use_qlora is True)
        if qlora_support is not None:
            model_load_kwargs["device_map"] = {"": args.device}
            model_load_kwargs["quantization_config"] = qlora_support.BitsAndBytesConfig(
                load_in_4bit=True,
                bnb_4bit_quant_type=args.qlora_quant_type,
                bnb_4bit_use_double_quant=args.qlora_double_quant,
                bnb_4bit_compute_dtype=torch_module.bfloat16,
            )

    return TrainingRuntimeOptions(
        is_cpu=False,
        use_qlora=qlora_support is not None,
        accelerator_kwargs={
            **accelerator_kwargs,
            "mixed_precision": "bf16",
        },
        model_load_kwargs=model_load_kwargs,
        qlora_support=qlora_support,
    )


def load_training_rows(train_jsonl: str) -> list[dict[str, object]]:
    with open(train_jsonl, "r", encoding="utf-8") as file:
        train_data = file.readlines()
    return [json.loads(line) for line in train_data if line.strip()]


def freeze_speaker_encoder(model) -> None:
    speaker_encoder = getattr(model, "speaker_encoder", None)
    if speaker_encoder is None:
        return

    speaker_encoder.requires_grad_(False)
    speaker_encoder.eval()


def parse_qlora_dropout(qlora_dropout: str) -> float:
    normalized_dropout = qlora_dropout.strip()
    if not normalized_dropout:
        raise ValueError("qlora_dropout must not be empty")

    try:
        parsed_dropout = float(normalized_dropout)
    except ValueError as exc:
        raise ValueError("qlora_dropout must be a valid number") from exc

    if parsed_dropout < 0 or parsed_dropout > 1:
        raise ValueError("qlora_dropout must be between 0 and 1")

    return parsed_dropout


def prepare_qlora_model(model, args: argparse.Namespace, runtime: TrainingRuntimeOptions):
    qlora_support = runtime.qlora_support
    if qlora_support is None:
        return model

    model = qlora_support.prepare_model_for_kbit_training(model)
    freeze_speaker_encoder(model)
    lora_config = qlora_support.LoraConfig(
        r=args.qlora_r,
        lora_alpha=args.qlora_alpha,
        lora_dropout=parse_qlora_dropout(args.qlora_dropout),
        bias="none",
        target_modules=DEFAULT_QLORA_TARGET_MODULES,
    )
    return qlora_support.get_peft_model(model, lora_config)


def count_parameters(model) -> tuple[int, int]:
    trainable_parameters = 0
    total_parameters = 0
    for parameter in model.parameters():
        parameter_count = parameter.numel()
        total_parameters += parameter_count
        if parameter.requires_grad:
            trainable_parameters += parameter_count
    return trainable_parameters, total_parameters


def initialize_training_pipeline(
    args: argparse.Namespace,
    dependencies: SimpleNamespace | None = None,
) -> TrainingPipelineContext:
    deps = dependencies or load_training_dependencies()
    runtime = build_runtime_options(args, deps.torch)
    accelerator = deps.Accelerator(**runtime.accelerator_kwargs)

    qwen3tts = deps.Qwen3TTSModel.from_pretrained(
        args.init_model_path,
        **runtime.model_load_kwargs,
    )
    freeze_speaker_encoder(qwen3tts.model)
    if runtime.use_qlora:
        qwen3tts.model = prepare_qlora_model(qwen3tts.model, args, runtime)

    config = deps.AutoConfig.from_pretrained(args.init_model_path)
    train_data = load_training_rows(args.train_jsonl)

    dataset = deps.TTSDataset(train_data, qwen3tts.processor, config)
    train_dataloader = deps.DataLoader(
        dataset,
        batch_size=args.batch_size,
        shuffle=True,
        collate_fn=dataset.collate_fn,
    )

    optimizer = deps.AdamW(
        (parameter for parameter in qwen3tts.model.parameters() if parameter.requires_grad),
        lr=args.lr,
        weight_decay=0.01,
    )
    if runtime.use_qlora:
        optimizer, train_dataloader = accelerator.prepare(
            optimizer,
            train_dataloader,
        )
        model = qwen3tts.model
    else:
        model, optimizer, train_dataloader = accelerator.prepare(
            qwen3tts.model,
            optimizer,
            train_dataloader,
        )

    trainable_parameters, total_parameters = count_parameters(model)
    mode_name = "QLoRA" if runtime.use_qlora else "full fine-tune"
    print(f"Training mode: {mode_name} | Trainable params: {trainable_parameters:,} / {total_parameters:,}")

    return TrainingPipelineContext(
        runtime=runtime,
        accelerator=accelerator,
        qwen3tts=qwen3tts,
        model=model,
        optimizer=optimizer,
        train_dataloader=train_dataloader,
        train_data=train_data,
    )


def get_model_device(model) -> object:
    if hasattr(model, "device"):
        return model.device
    first_param = next(model.parameters())
    return first_param.device


def get_model_dtype(model, default_dtype) -> object:
    if hasattr(model, "dtype"):
        return model.dtype
    first_param = next(model.parameters())
    return getattr(first_param, "dtype", default_dtype)


def run_training(args: argparse.Namespace, dependencies: SimpleNamespace | None = None) -> None:
    global target_speaker_embedding
    target_speaker_embedding = None

    deps = dependencies or load_training_dependencies()
    context = initialize_training_pipeline(args, deps)
    speaker_name = normalize_speaker_name(args.speaker_name)
    accelerator = context.accelerator
    model = context.model
    optimizer = context.optimizer
    train_dataloader = context.train_dataloader
    model_device = get_model_device(model)
    model_dtype = get_model_dtype(model, context.runtime.model_load_kwargs["torch_dtype"])

    num_epochs = args.num_epochs
    model.train()

    for epoch in range(num_epochs):
        for step, batch in enumerate(train_dataloader):
            with accelerator.accumulate(model):
                input_ids = batch["input_ids"]
                codec_ids = batch["codec_ids"]
                ref_mels = batch["ref_mels"]
                text_embedding_mask = batch["text_embedding_mask"]
                codec_embedding_mask = batch["codec_embedding_mask"]
                attention_mask = batch["attention_mask"]
                codec_0_labels = batch["codec_0_labels"]
                codec_mask = batch["codec_mask"]

                with deps.torch.no_grad():
                    speaker_embedding = model.speaker_encoder(ref_mels.to(model_device).to(model_dtype)).detach()
                if target_speaker_embedding is None:
                    target_speaker_embedding = speaker_embedding

                input_text_ids = input_ids[:, :, 0]
                input_codec_ids = input_ids[:, :, 1]

                input_text_embedding = model.talker.model.text_embedding(input_text_ids) * text_embedding_mask
                input_codec_embedding = model.talker.model.codec_embedding(input_codec_ids) * codec_embedding_mask
                input_codec_embedding[:, 6, :] = speaker_embedding

                input_embeddings = input_text_embedding + input_codec_embedding

                for i in range(1, 16):
                    codec_i_embedding = model.talker.code_predictor.get_input_embeddings()[i - 1](codec_ids[:, :, i])
                    codec_i_embedding = codec_i_embedding * codec_mask.unsqueeze(-1)
                    input_embeddings = input_embeddings + codec_i_embedding

                outputs = model.talker(
                    inputs_embeds=input_embeddings[:, :-1, :],
                    attention_mask=attention_mask[:, :-1],
                    labels=codec_0_labels[:, 1:],
                    output_hidden_states=True,
                )

                hidden_states = outputs.hidden_states[0][-1]
                talker_hidden_states = hidden_states[codec_mask[:, :-1]]
                talker_codec_ids = codec_ids[codec_mask]

                _, sub_talker_loss = model.talker.forward_sub_talker_finetune(
                    talker_codec_ids,
                    talker_hidden_states,
                )

                loss = outputs.loss + 0.3 * sub_talker_loss

                accelerator.backward(loss)

                if accelerator.sync_gradients:
                    accelerator.clip_grad_norm_(model.parameters(), 1.0)

                optimizer.step()
                optimizer.zero_grad()

            if step % 10 == 0:
                print(f"Epoch {epoch} | Step {step} | Loss: {loss.item():.4f}")

        accelerator.wait_for_everyone()
        should_export_checkpoint = (not context.runtime.use_qlora) or epoch == num_epochs - 1
        if accelerator.is_main_process and should_export_checkpoint:
            output_dir = os.path.join(args.output_model_path, f"checkpoint-epoch-{epoch}")
            shutil.copytree(args.init_model_path, output_dir, dirs_exist_ok=True)

            input_config_file = os.path.join(args.init_model_path, "config.json")
            output_config_file = os.path.join(output_dir, "config.json")
            with open(input_config_file, "r", encoding="utf-8") as f:
                config_dict = json.load(f)
            config_dict["tts_model_type"] = "custom_voice"
            talker_config = config_dict.get("talker_config", {})
            talker_config["spk_id"] = {
                speaker_name: 3000,
            }
            talker_config["spk_is_dialect"] = {
                speaker_name: False,
            }
            config_dict["talker_config"] = talker_config

            with open(output_config_file, "w", encoding="utf-8") as f:
                json.dump(config_dict, f, indent=2, ensure_ascii=False)

            unwrapped_model = accelerator.unwrap_model(model)
            export_model = unwrapped_model
            if context.runtime.use_qlora and hasattr(unwrapped_model, "merge_and_unload"):
                export_model = unwrapped_model.merge_and_unload()

            state_dict = {k: v.detach().to("cpu") for k, v in export_model.state_dict().items()}

            drop_prefix = "speaker_encoder"
            keys_to_drop = [k for k in state_dict.keys() if k.startswith(drop_prefix)]
            for k in keys_to_drop:
                del state_dict[k]

            weight = state_dict["talker.model.codec_embedding.weight"]
            state_dict["talker.model.codec_embedding.weight"][3000] = target_speaker_embedding[0].detach().to(weight.device).to(weight.dtype)
            save_path = os.path.join(output_dir, "model.safetensors")
            deps.save_file(state_dict, save_path)


def train(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    run_training(args)


if __name__ == "__main__":
    train()