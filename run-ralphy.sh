#!/bin/bash

# Run ralphy with codex in dangerous mode, logging to file
LOG_DIR="$HOME/code/pdf-editor/logs"
mkdir -p "$LOG_DIR"

TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="$LOG_DIR/ralphy_${TIMESTAMP}.log"

echo "Starting ralphy at $(date)" | tee "$LOG_FILE"
echo "Log file: $LOG_FILE"
echo "---" | tee -a "$LOG_FILE"

ralphy --codex -v 2>&1 | tee -a "$LOG_FILE"

echo "---" | tee -a "$LOG_FILE"
echo "Finished at $(date)" | tee -a "$LOG_FILE"
