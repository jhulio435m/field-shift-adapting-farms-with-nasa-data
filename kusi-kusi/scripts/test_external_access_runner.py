"""Failure reporting tests; these do not replace PostgreSQL/HTTP acceptance."""
from contextlib import redirect_stdout
import errno
import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import verify_external_access as runner


class RunnerReportingTests(unittest.TestCase):
    def invoke(self, error=None, result=None):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "result.json"
            output = io.StringIO()
            with patch.object(runner, "verify", side_effect=error, return_value=result):
                with redirect_stdout(output):
                    code = runner.main(["--result", str(path)])
            artifact = json.loads(path.read_text())
            self.assertEqual(json.loads(output.getvalue()), artifact)
            return code, artifact, output.getvalue()

    def test_permission_denied_writes_blocked_artifact_and_returns_two(self):
        code, result, output = self.invoke(error=PermissionError(errno.EPERM, "private-context"))
        self.assertEqual(code, 2)
        self.assertEqual(result["status"], "blocked")
        self.assertFalse(result["verification_complete"])
        self.assertNotIn("private-context", output)

    def test_subprocess_failure_does_not_disclose_arguments_or_output(self):
        error = subprocess.CalledProcessError(1, ["command", "private-argument"], output="private-output")
        code, result, output = self.invoke(error=error)
        self.assertEqual(code, 1)
        self.assertEqual(result["status"], "failed")
        self.assertNotIn("private-argument", output)
        self.assertNotIn("private-output", output)

    def test_failed_check_does_not_reuse_a_success_result(self):
        code, result, output = self.invoke(error=RuntimeError("private-response"))
        self.assertEqual(code, 1)
        self.assertFalse(result["verification_complete"])
        self.assertNotIn("private-response", output)

    def test_missing_executable_is_failed_rather_than_permission_blocked(self):
        code, result, _ = self.invoke(error=FileNotFoundError(errno.ENOENT, "private-path"))
        self.assertEqual(code, 1)
        self.assertEqual(result["reason"], "operating_system_error")

    def test_success_preserves_actual_verifier_evidence(self):
        evidence = {"status": "passed", "checks": ["synthetic-test-marker"]}
        code, result, _ = self.invoke(result=evidence)
        self.assertEqual(code, 0)
        self.assertEqual(result, evidence)


if __name__ == "__main__":
    unittest.main()
