#!/usr/bin/env bash
~/code/ralphy/ralphy.sh --prd PRD.md --no-tests -v "$@" 2>&1 | tee "logs/ralphy_$(date +%Y%m%d_%H%M%S).log"
