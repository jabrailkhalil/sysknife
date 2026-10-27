"""Check each original incorrect claim independently against the real Rust test."""

from pathlib import Path
import subprocess

path = Path("crates/sysknife-brain/src/prompt.rs")
fixed_source = path.read_text()
reversions = [
    (
        "- `AptAutoremove` removes orphaned dependency packages — MEDIUM: apt chooses the removal set at run time, which can include old kernels.",
        "- `AptAutoremove` removes orphaned dependency packages only — LOW.",
    ),
    (
        "- `AptAutoremove` is MEDIUM — apt chooses the removal set at run time, which can include old kernels.",
        "- `AptAutoremove` is LOW, not HIGH — only removes orphaned packages that nothing depends on.",
    ),
]
try:
    for index, (fixed, old) in enumerate(reversions, 1):
        assert fixed_source.count(fixed) == 1
        path.write_text(fixed_source.replace(fixed, old))
        result = subprocess.run(
            ["cargo", "test", "-p", "sysknife-daemon", "--test", "prompt_risk_labels",
             "--locked", "prose_risk_claims_match_the_action_specs", "--", "--exact", "--nocapture"],
            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        )
        Path(f"../evidence/revert-line-{index}.log").write_text(result.stdout)
        assert result.returncode == 101, result.stdout
        assert "AptAutoremove: prose says Low, ActionSpec says Medium" in result.stdout
        assert "test result: FAILED. 0 passed; 1 failed;" in result.stdout
        print(f"reversion {index}: expected AptAutoremove assertion failure confirmed")
finally:
    path.write_text(fixed_source)
