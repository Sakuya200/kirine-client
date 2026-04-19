import importlib
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]

if str(SRC_MODEL_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_MODEL_ROOT))


def load_module():
    module_name = "vox_cpm2.training"
    sys.modules.pop(module_name, None)
    return importlib.import_module(module_name)


class VoxCpm2TrainingScriptResolutionTests(unittest.TestCase):
    def test_resolve_train_script_path_uses_vendor_checkout_when_available(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_training_path = temp_root / "src-model" / "vox_cpm2" / "training.py"
            fake_training_path.parent.mkdir(parents=True, exist_ok=True)
            fake_training_path.write_text("# stub", encoding="utf-8")

            vendor_script = temp_root / "src-model" / "vendor" / "VoxCPM" / "scripts" / "train_voxcpm_finetune.py"
            vendor_script.parent.mkdir(parents=True, exist_ok=True)
            vendor_script.write_text("# train", encoding="utf-8")

            with patch.object(module, "__file__", str(fake_training_path)), patch.object(
                module.importlib.util, "find_spec", return_value=None
            ):
                resolved = module.resolve_train_script_path("")

            self.assertEqual(resolved, vendor_script.resolve())

    def test_resolve_train_script_path_bootstraps_vendor_sources_when_missing(self):
        module = load_module()

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            fake_training_path = temp_root / "src-model" / "vox_cpm2" / "training.py"
            fake_training_path.parent.mkdir(parents=True, exist_ok=True)
            fake_training_path.write_text("# stub", encoding="utf-8")

            vendor_script = temp_root / "src-model" / "vendor" / "VoxCPM" / "scripts" / "train_voxcpm_finetune.py"

            def fake_bootstrap() -> Path:
                vendor_script.parent.mkdir(parents=True, exist_ok=True)
                vendor_script.write_text("# train", encoding="utf-8")
                return vendor_script

            with patch.object(module, "__file__", str(fake_training_path)), patch.object(
                module.importlib.util, "find_spec", return_value=None
            ), patch.object(module, "ensure_voxcpm_training_sources", side_effect=fake_bootstrap):
                resolved = module.resolve_train_script_path("")

            self.assertEqual(resolved, vendor_script.resolve())


if __name__ == "__main__":
    unittest.main()