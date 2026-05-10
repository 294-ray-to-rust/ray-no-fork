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

# Use per-slot artifact directory if configured (avoids cross-slot
# collisions when multiple agent slots share a host).
ARTIFACT_DIR="${RAYCI_ARTIFACT_DIR:-/tmp/artifacts}"
mkdir -p "$ARTIFACT_DIR"

echo "--- :gear: Generating pipeline"

# Select pipeline directory based on branch type.
# PRs run the RayRust fast-iteration gate: forge, lint, packaging
# sanity, and the representative canary manifest.
# Merge queue is intentionally lightweight; PR CI is the real gate and
# main remains the full authoritative suite.
case "${BUILDKITE_BRANCH:-}" in
  main|get-the-build-working)
    PIPELINE_DIR=".buildkite/fork-pipeline/"
    echo "Branch '${BUILDKITE_BRANCH}': using FULL pipeline"
    ;;
  gh-readonly-queue/*)
    PIPELINE_DIR=".buildkite/fork-pipeline-mq/"
    echo "Branch '${BUILDKITE_BRANCH}': using lightweight merge-queue sanity pipeline"
    ;;
  *)
    PIPELINE_DIR=".buildkite/fork-pipeline-pr/"
    echo "Branch '${BUILDKITE_BRANCH:-unknown}': using PR pipeline (canary gate + packaging sanity)"
    ;;
esac

rayci -output "$ARTIFACT_DIR/pipeline.yaml" \
  -config .buildkite/fork-config.yaml \
  -buildkite-dir "$PIPELINE_DIR"

STEP_COUNT=$(grep -c "key:" "$ARTIFACT_DIR/pipeline.yaml" || echo 0)
GROUP_COUNT=$(grep -c "group:" "$ARTIFACT_DIR/pipeline.yaml" || echo 0)
echo "Generated pipeline: $STEP_COUNT steps across $GROUP_COUNT groups"

if [ "$STEP_COUNT" -eq 0 ]; then
  echo "ERROR: No pipeline steps generated!"
  exit 1
fi

echo "--- :page_facing_up: Pipeline YAML preview (first 20 lines)"
head -20 "$ARTIFACT_DIR/pipeline.yaml"
echo "--- :page_facing_up: Pipeline YAML preview (last 20 lines)"
tail -20 "$ARTIFACT_DIR/pipeline.yaml"

if [ "${RAYCI_SKIP_FLATTEN:-0}" = "1" ]; then
  echo "RAYCI_SKIP_FLATTEN=1: Skipping group flattening, uploading original grouped YAML"
  cp "$ARTIFACT_DIR/pipeline.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
  FLAT_STEP_COUNT=$STEP_COUNT
else
  echo "--- :wrench: Installing yq"
  if ! command -v yq &>/dev/null; then
    YQ_BIN="/tmp/yq"
    YQ_URL="https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64"
    YQ_AVAILABLE=0
    if command -v curl &>/dev/null; then
      if curl -fsSL -o "$YQ_BIN" "$YQ_URL" 2>/dev/null; then
        YQ_AVAILABLE=1
      else
        echo "WARNING: yq download failed (curl), skipping flattening."
      fi
    elif command -v wget &>/dev/null; then
      if wget -qO "$YQ_BIN" "$YQ_URL" 2>/dev/null; then
        YQ_AVAILABLE=1
      else
        echo "WARNING: yq download failed (wget), skipping flattening."
      fi
    else
      echo "WARNING: cannot download yq (no curl or wget)"
    fi
    if [ "$YQ_AVAILABLE" = "1" ]; then
      chmod +x "$YQ_BIN"
      export PATH="/tmp:$PATH"
    else
      echo "Falling back to uploading original grouped YAML (no flattening)."
      cp "$ARTIFACT_DIR/pipeline.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
      FLAT_STEP_COUNT=$STEP_COUNT
    fi
  fi

  if command -v yq &>/dev/null; then
    yq --version

    echo "--- :scissors: Flattening group blocks"
    # The yq expression below flattens group blocks: it extracts steps
    # from inside groups. NOTE: group-level depends_on is NOT propagated
    # to child steps — each step must have its own depends_on.
    # If the expression fails (e.g. yq version
    # incompatibility), fall back to the original grouped YAML.
    if yq '
      .steps = [
        .steps[] |
        select(has("group")) .steps[] // select(has("group") | not)
      ]
    ' "$ARTIFACT_DIR/pipeline.yaml" > "$ARTIFACT_DIR/pipeline_flat.yaml" 2>"$ARTIFACT_DIR/yq_stderr.txt"; then
      echo "yq flattening succeeded"
    else
      echo "WARNING: yq flattening failed, falling back to original pipeline."
      if [ -s "$ARTIFACT_DIR/yq_stderr.txt" ]; then
        cat "$ARTIFACT_DIR/yq_stderr.txt"
      fi
      cp "$ARTIFACT_DIR/pipeline.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
    fi

    echo "--- :mag: Validating flattened pipeline"
    yq eval '.' "$ARTIFACT_DIR/pipeline_flat.yaml" > /dev/null 2>&1 || {
      echo "ERROR: Flattened YAML is not valid! Falling back to original."
      cp "$ARTIFACT_DIR/pipeline.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
    }

    ORIG_STEP_COUNT=$(yq '.steps | length' "$ARTIFACT_DIR/pipeline.yaml")
    FLAT_STEP_COUNT=$(yq '.steps | length' "$ARTIFACT_DIR/pipeline_flat.yaml")
    echo "Step counts: original=$ORIG_STEP_COUNT, flattened=$FLAT_STEP_COUNT"

    if [ "$FLAT_STEP_COUNT" -eq 0 ]; then
      echo "ERROR: Flattened pipeline has 0 steps! Falling back to original."
      cp "$ARTIFACT_DIR/pipeline.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
      FLAT_STEP_COUNT=$ORIG_STEP_COUNT
    fi

    echo "--- :no_entry: Disabling automatic retries for Rust PR preflight steps"
    if yq '
      .steps |= map(
        if ((.label // "") | test("^(wanda: rust|:ray: rust )")) then
          .retry.automatic = false
        else
          .
        end
      )
    ' "$ARTIFACT_DIR/pipeline_flat.yaml" > "$ARTIFACT_DIR/pipeline_no_retries.yaml" 2>"$ARTIFACT_DIR/retry_yq_stderr.txt"; then
      mv "$ARTIFACT_DIR/pipeline_no_retries.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
    else
      echo "WARNING: could not disable Rust step retries; leaving pipeline unchanged."
      if [ -s "$ARTIFACT_DIR/retry_yq_stderr.txt" ]; then
        cat "$ARTIFACT_DIR/retry_yq_stderr.txt"
      fi
    fi

    DID_FLATTEN=1

    echo "--- :page_facing_up: Diagnostic diff (first step before/after flattening)"
    diff <(yq '.steps[0]' "$ARTIFACT_DIR/pipeline.yaml") \
         <(yq '.steps[0]' "$ARTIFACT_DIR/pipeline_flat.yaml") \
         > "$ARTIFACT_DIR/flatten_diff.txt" 2>&1 || true
  else
    echo "yq not available, skipping flattening."
    cp "$ARTIFACT_DIR/pipeline.yaml" "$ARTIFACT_DIR/pipeline_flat.yaml"
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
buildkite-agent pipeline upload "$ARTIFACT_DIR/pipeline_flat.yaml" 2>"$ARTIFACT_DIR/upload_stderr.txt"
if [ -s "$ARTIFACT_DIR/upload_stderr.txt" ]; then
  echo "--- :warning: pipeline upload stderr"
  cat "$ARTIFACT_DIR/upload_stderr.txt"
fi

if [ "${DID_FLATTEN:-0}" = "1" ]; then
  buildkite-agent annotate \
    "Pipeline bootstrap: uploaded $FLAT_STEP_COUNT steps flattened from $GROUP_COUNT groups ($(wc -l < "$ARTIFACT_DIR/pipeline_flat.yaml") lines of YAML)." \
    --style success --context pipeline-info
else
  buildkite-agent annotate \
    "Pipeline bootstrap: uploaded $FLAT_STEP_COUNT steps (no flattening, $(wc -l < "$ARTIFACT_DIR/pipeline_flat.yaml") lines of YAML)." \
    --style success --context pipeline-info
fi

echo "Pipeline uploaded successfully ($FLAT_STEP_COUNT steps)"
