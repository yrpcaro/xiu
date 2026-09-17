#!/usr/bin/env python3
import io
import json
import struct
import subprocess
import sys
from pathlib import Path
import unittest
from unittest.mock import MagicMock

host_path = Path(__file__).parent / "xiufox-host.py"


class TestXiufoxHost(unittest.TestCase):
    def test_stdin_eof_causes_clean_exit(self):
        # When stdin is empty (EOF immediately), xiufox-host must exit 0 promptly
        proc = subprocess.run(
            [sys.executable, str(host_path)],
            input=b"",
            capture_output=True,
            timeout=5,
        )
        self.assertEqual(proc.returncode, 0)

    def test_send_wire_format(self):
        payload = {"primary": "#ff0000", "surface": "#000000"}
        encoded = json.dumps(payload).encode()

        import importlib.machinery
        import importlib.util

        loader = importlib.machinery.SourceFileLoader("xiufox_host", str(host_path))
        spec = importlib.util.spec_from_loader("xiufox_host", loader)
        mod = importlib.util.module_from_spec(spec)
        loader.exec_module(mod)

        fake_out = io.BytesIO()
        mock_stdout = MagicMock()
        mock_stdout.buffer = fake_out
        orig_stdout = sys.stdout
        try:
            sys.stdout = mock_stdout
            mod.send(payload)
        finally:
            sys.stdout = orig_stdout

        wire = fake_out.getvalue()
        msg_len = struct.unpack("<I", wire[:4])[0]
        self.assertEqual(msg_len, len(encoded))
        self.assertEqual(json.loads(wire[4:4+msg_len].decode()), payload)


if __name__ == "__main__":
    unittest.main()
