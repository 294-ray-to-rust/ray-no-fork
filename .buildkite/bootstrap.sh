#!/usr/bin/env bash
# Bootstrap script for the Buildkite pipeline.
# Generates the pipeline YAML via rayci, optionally flattens group blocks,
# and uploads the result.
#
# Extracted from pipeline.yml so that Buildkite's interpolation engine
# does not need to process shell $ patterns.

set -euo pipefail

echo "--- :buildkite: Agent info"
buildkite-agent --version

mkdir -p /tmp/artifacts

echo "--- :gear: Generating pipeline"
rayci -output /tmp/artifacts/pipeline.yaml \
  -config .buildkite/fork-config.yaml \
  -buildkite-dir .buildkite/fork-pipeline/

STEP_COUNT=$(grep -c "key:" /tmp/artifacts/pipeline.yaml || echo 0)
GROUP_COUNT=$(grep -c "group:" /tmp/artifacts/pipeline.yaml || echo 0)
echo "Generated pipeline: $STEP_COUNT steps across $GROUP_COUNT groups"

if [ "$STEP_COUNT" -eq 0 ]; then
  echo "ERROR: No pipeline steps generated!"
  exit 1
fi

echo "--- :page_facing_up: Pipeline YAML preview (first 20 lines)"
head -20 /tmp/artifacts/pipeline.yaml
echo "--- :page_facing_up: Pipeline YAML preview (last 20 lines)"
tail -20 /tmp/artifacts/pipeline.yaml

if [ "${RAYCI_SKIP_FLATTEN:-0}" = "1" ]; then
  echo "RAYCI_SKIP_FLATTEN=1: Skipping group flattening, uploading original grouped YAML"
  cp /tmp/artifacts/pipeline.yaml /tmp/artifacts/pipeline_flat.yaml
  FLAT_STEP_COUNT=$STEP_COUNT
else
  echo "--- :wrench: Installing yq"
  if ! command -v yq &>/dev/null; then
    YQ_BIN="/tmp/yq"
    YQ_URL="https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64"
    if command -v curl &>/dev/null; then
      curl -fsSL -o "$YQ_BIN" "$YQ_URL"
    elif command -v wget &>/dev/null; then
      wget -qO "$YQ_BIN" "$YQ_URL"
    elif command -v nix-shell &>/dev/null; then
      echo "Using nix-shell to download yq"
      nix-shell -p curl --run "curl -fsSL -o '$YQ_BIN' '$YQ_URL'"
    else
      echo "WARNING: cannot download yq (no curl, wget, or nix-shell)"
      echo "Falling back to uploading original grouped YAML (no flattening)."
      cp /tmp/artifacts/pipeline.yaml /tmp/artifacts/pipeline_flat.yaml
      FLAT_STEP_COUNT=$STEP_COUNT
      YQ_AVAILABLE=0
    fi
    if [ "${YQ_AVAILABLE:-1}" = "1" ]; then
      chmod +x "$YQ_BIN"
      export PATH="/tmp:$PATH"
    fi
  fi

  if command -v yq &>/dev/null; then
    yq --version

    echo "--- :scissors: Flattening group blocks"
    yq '
      .steps = [
        .steps[] |
        if has("group") then
          .as $group |
          .steps[] |
          . * (
            if ((."depends_on" // []) + ($group."depends_on" // []) | length) > 0
            then {"depends_on": ((."depends_on" // []) + ($group."depends_on" // []) | unique)}
            else {}
            end
          )
        else
          .
        end
      ]
    ' /tmp/artifacts/pipeline.yaml > /tmp/artifacts/pipeline_flat.yaml

    echo "--- :mag: Validating flattened pipeline"
    yq eval '.' /tmp/artifacts/pipeline_flat.yaml > /dev/null 2>&1 || {
      echo "ERROR: Flattened YAML is not valid! Falling back to original."
      cp /tmp/artifacts/pipeline.yaml /tmp/artifacts/pipeline_flat.yaml
    }

    ORIG_STEP_COUNT=$(yq '.steps | length' /tmp/artifacts/pipeline.yaml)
    FLAT_STEP_COUNT=$(yq '.steps | length' /tmp/artifacts/pipeline_flat.yaml)
    echo "Step counts: original=$ORIG_STEP_COUNT, flattened=$FLAT_STEP_COUNT"

    if [ "$FLAT_STEP_COUNT" -eq 0 ]; then
      echo "ERROR: Flattened pipeline has 0 steps! Falling back to original."
      cp /tmp/artifacts/pipeline.yaml /tmp/artifacts/pipeline_flat.yaml
      FLAT_STEP_COUNT=$ORIG_STEP_COUNT
    fi

    DID_FLATTEN=1

    echo "--- :page_facing_up: Diagnostic diff (first step before/after flattening)"
    diff <(yq '.steps[0]' /tmp/artifacts/pipeline.yaml) \
         <(yq '.steps[0]' /tmp/artifacts/pipeline_flat.yaml) \
         > /tmp/artifacts/flatten_diff.txt 2>&1 || true
  else
    echo "yq not available, skipping flattening."
    cp /tmp/artifacts/pipeline.yaml /tmp/artifacts/pipeline_flat.yaml
    FLAT_STEP_COUNT=$STEP_COUNT
  fi
fi

echo "--- :buildkite: Uploading pipeline"
# Do NOT use --no-interpolation here. The rayci-generated YAML (from
# fork-pipeline/*.rayci.yml) uses Buildkite's ${ } escape syntax for
# variables that must resolve at step execution time (e.g.
# ${BUILDKITE_PARALLEL_JOB_COUNT}, ${RAYCI_WORK_REPO}). Buildkite
# interpolation converts ${ } at upload time so the shell can
# evaluate them when the step runs.
# See: https://github.com/294-ray-to-rust/ray-no-fork/issues/149
buildkite-agent pipeline upload /tmp/artifacts/pipeline_flat.yaml 2>/tmp/artifacts/upload_stderr.txt
if [ -s /tmp/artifacts/upload_stderr.txt ]; then
  echo "--- :warning: pipeline upload stderr"
  cat /tmp/artifacts/upload_stderr.txt
fi

if [ "${DID_FLATTEN:-0}" = "1" ]; then
  buildkite-agent annotate \
    "Pipeline bootstrap: uploaded $FLAT_STEP_COUNT steps flattened from $GROUP_COUNT groups ($(wc -l < /tmp/artifacts/pipeline_flat.yaml) lines of YAML)." \
    --style success --context pipeline-info
else
  buildkite-agent annotate \
    "Pipeline bootstrap: uploaded $FLAT_STEP_COUNT steps (no flattening, $(wc -l < /tmp/artifacts/pipeline_flat.yaml) lines of YAML)." \
    --style success --context pipeline-info
fi

echo "Pipeline uploaded successfully ($FLAT_STEP_COUNT steps)"
