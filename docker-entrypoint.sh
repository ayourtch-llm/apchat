#!/bin/bash
# Docker entrypoint wrapper for apchat
# Loops around apchat invocations, with heartbeat between restarts

while true; do
    /usr/local/bin/apchat "$@"
    echo "[$(date)] apchat exited. Restarting..."
    sleep 1
done