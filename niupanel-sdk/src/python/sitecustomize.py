import builtins
import os


def _enabled() -> bool:
    value = os.environ.get("NIUPANEL_SDK_AUTO_GLOBALS", "1").strip().lower()
    return value not in {"0", "false", "no", "off"}


if _enabled() and not hasattr(builtins, "niu"):
    try:
        from niu import niu as _niu

        builtins.niu = _niu
    except Exception:
        pass
