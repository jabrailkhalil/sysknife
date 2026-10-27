#!/usr/bin/env bash
set -euo pipefail
arm="$1"
mkdir -p "../evidence/${arm}-initialization"
scp -q -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
  -i "$SYSKNIFE_VM_SSH_KEY" -o IdentitiesOnly=yes -o BatchMode=yes -P 2223 \
  ../harness/validation/warm-prefix.rs ubuntu@127.0.0.1:/tmp/pr695-warm-prefix.rs
bash tests/e2e/ubuntu-vm.sh ssh 'bash -s' <<'GUEST'
set -euo pipefail
source /home/ubuntu/.cargo/env
cd /home/ubuntu/sysknife
mkdir -p /tmp/pr695-prefix-generator/src
cp /tmp/pr695-warm-prefix.rs /tmp/pr695-prefix-generator/src/main.rs
cp Cargo.lock /tmp/pr695-prefix-generator/Cargo.lock
cat > /tmp/pr695-prefix-generator/Cargo.toml <<'TOML'
[package]
name = "pr695-prefix-generator"
version = "0.0.0"
edition = "2021"
[dependencies]
sysknife-brain = { path = "/home/ubuntu/sysknife/crates/sysknife-brain" }
sysknife-types = { path = "/home/ubuntu/sysknife/crates/sysknife-types" }
async-trait = "0.1"
serde_json = "1"
TOML
# Only the temporary helper lock is bootstrapped; the production lock and
# source remain unchanged, and no new registry downloads are allowed.
CARGO_TARGET_DIR="$PWD/target" cargo build --release --offline \
  --manifest-path /tmp/pr695-prefix-generator/Cargo.toml
CARGO_TARGET_DIR="$PWD/target" cargo build --release --offline --locked \
  --manifest-path /tmp/pr695-prefix-generator/Cargo.toml
git diff --exit-code
target/release/pr695-prefix-generator > /tmp/pr695-warm-prefix.json
GUEST
bash tests/e2e/ubuntu-vm.sh ssh 'cat /tmp/pr695-warm-prefix.json' \
  > "../evidence/${arm}-initialization/request.json"
date --iso-8601=seconds > "../evidence/${arm}-initialization/started.txt"
# Initialization has its own explicit budget; it is not a story run or a pass.
curl --fail --silent --show-error --max-time 1800 -H 'Content-Type: application/json' \
  --data-binary "@../evidence/${arm}-initialization/request.json" \
  http://127.0.0.1:11434/api/chat > "../evidence/${arm}-initialization/response.json"
date --iso-8601=seconds > "../evidence/${arm}-initialization/completed.txt"
python3 - "$arm" <<'PY'
import hashlib, json, pathlib, sys
root = pathlib.Path('../evidence') / (sys.argv[1] + '-initialization')
request = json.loads((root / 'request.json').read_text())
response = json.loads((root / 'response.json').read_text())
assert request['messages'][-1]['content'] == 'Acknowledge initialization briefly.'
assert request['options']['num_predict'] == 1
assert response['done'] is True, response
assert response['prompt_eval_count'] > 0, response
digest = hashlib.sha256(request['messages'][0]['content'].encode()).hexdigest()
(root / 'system-prompt.sha256').write_text(digest + '\n')
print(json.dumps({'arm': sys.argv[1], 'initialization_only': True,
                  'prompt_eval_count': response['prompt_eval_count'],
                  'eval_count': response.get('eval_count'),
                  'system_prompt_sha256': digest}, indent=2))
PY
