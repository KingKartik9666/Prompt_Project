#!/bin/sh
# Modes:
#   selfcheck              (default) simulator self-check + repair-retry test + saved test table. No API key needed.
#   ask "<question>" [level]   one live run of the full pipeline (needs GEMINI_API_KEY)
#   tests                  all 13 test cases on Gemini (needs GEMINI_API_KEY; uses ~40 requests)
#   versions               explainer v1/v2/v3 comparison on Gemini (needs GEMINI_API_KEY)
#   notebook               JupyterLab on http://localhost:8888 (open code/s18_workflow.ipynb)
#   bash                   a shell inside the container
set -e
cd /app/code
mode="${1:-selfcheck}"
[ $# -gt 0 ] && shift

need_key() {
  if [ -z "$GEMINI_API_KEY" ]; then
    echo "GEMINI_API_KEY is not set. Run with:  docker run --rm -e GEMINI_API_KEY=<your key> s18-tutor $mode ..."
    echo "Without a key, use:  docker run --rm s18-tutor selfcheck"
    exit 2
  fi
}

case "$mode" in
  selfcheck)
    echo "== 1. Simulator self-check (NumPy ground truth; wrong claims must be flagged)"
    python quantum_sim.py
    echo
    echo "== 2. Repair-retry control-flow test (scripted model answers, no API)"
    python test_repair.py
    echo
    echo "== 3. Saved test table: Gemini 3.5 Flash-Lite run (automatic checks)"
    cat ../test-data/outputs/test_table_20261122_1710.md
    echo
    echo "Live run:   docker run --rm -e GEMINI_API_KEY=<key> s18-tutor ask \"How do you create a Bell state?\""
    echo "Notebook:   docker run --rm -p 8888:8888 -e GEMINI_API_KEY=<key> s18-tutor notebook"
    ;;
  ask)
    need_key
    q="${1:-How do you create a Bell state?}"
    level="${2:-beginner}"
    python -c "import json,sys; from workflow import run; print(json.dumps(run(sys.argv[1], sys.argv[2]), indent=2, ensure_ascii=False))" "$q" "$level"
    ;;
  tests)
    need_key
    python run_tests.py "$@"
    ;;
  versions)
    need_key
    python compare_versions.py "$@"
    ;;
  notebook)
    [ -z "$GEMINI_API_KEY" ] && echo "Note: no GEMINI_API_KEY set; simulator cells and saved outputs still work."
    exec jupyter lab --ip=0.0.0.0 --port=8888 --no-browser --ServerApp.root_dir=/app \
      --ServerApp.token='' --ServerApp.password='' --ServerApp.default_url=/lab/tree/code/s18_workflow.ipynb
    ;;
  bash|sh)
    exec /bin/sh
    ;;
  *)
    echo "Unknown mode '$mode'. Use: selfcheck | ask \"<question>\" [beginner|intermediate] | tests | versions | notebook | bash"
    exit 1
    ;;
esac
