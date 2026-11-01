#!/usr/bin/env python3
"""Print per-job durations for a workflow run, plus step-level detail on request."""
import json
import subprocess
import sys
from datetime import datetime

REPO = "Ray-D-Song/raster"


def fetch(run_id):
    out = subprocess.run(
        ["gh", "api", f"repos/{REPO}/actions/runs/{run_id}/jobs", "--paginate"],
        capture_output=True, text=True, check=True).stdout
    return json.loads(out)["jobs"]


def ts(value):
    return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")


def main():
    run_id = sys.argv[1]
    steps = "--steps" in sys.argv
    jobs = fetch(run_id)
    rows = {}
    for job in jobs:
        if not (job.get("started_at") and job.get("completed_at")):
            rows[job["name"]] = (job.get("conclusion") or "incomplete", None)
            continue
        rows[job["name"]] = (job["conclusion"],
                             int((ts(job["completed_at"]) - ts(job["started_at"])).total_seconds()))
        if steps:
            print(f"### {job['name']}  [{job['conclusion']}]")
            for step in job["steps"]:
                if step.get("completed_at"):
                    dur = int((ts(step["completed_at"]) - ts(step["started_at"])).total_seconds())
                else:
                    dur = "-"
                print(f"    {step['name']}\t{dur}")
    print(json.dumps({run_id: rows}, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
