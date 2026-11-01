#!/usr/bin/env python3
"""
H.I.V.E. Headless Screenshot Capture CLI (Python wrapper)
Author: Snapwave//Lushapps
Description: Dispatches batchmode Unity rendering tasks to capture high-fidelity
             cybernetic scene stills, workspace views, and agent focus renders.
"""

import os
import sys
import argparse
import subprocess
from pathlib import Path

# Ensure UTF-8 output on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass


DEFAULT_UNITY_PATHS = [
    r"C:\Program Files\Unity\Hub\Editor\6000.6.7f1\Editor\Unity.exe",
    r"C:\Program Files\Unity\Editor\Unity.exe"
]

SCENES = [
    "CommandChamber",
    "Workspaces",
    "CollabChamber",
    "KnowledgeArchive",
    "Infrastructure",
    "OwnerControl",
    "Atrium"
]

def find_unity_binary(custom_path=None):
    if custom_path and Path(custom_path).is_file():
        return custom_path
    
    env_path = os.environ.get("UNITY_PATH")
    if env_path and Path(env_path).is_file():
        return env_path
        
    for p in DEFAULT_UNITY_PATHS:
        if Path(p).is_file():
            return p
            
    # Search in Unity Hub default directory
    hub_root = Path(r"C:\Program Files\Unity\Hub\Editor")
    if hub_root.exists():
        found = list(hub_root.glob("*/Editor/Unity.exe"))
        if found:
            return str(found[0])
            
    return None

def main():
    parser = argparse.ArgumentParser(
        description="H.I.V.E. Headless Screenshot Capture Tool"
    )
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--all", action="store_true", help="Capture all operational scenes")
    group.add_argument("--scene", "-s", type=str, choices=SCENES, help="Specific scene to capture")

    parser.add_argument("--res", "-r", type=str, default="1920x1080", help="Resolution WxH (default: 1920x1080)")
    parser.add_argument("--out", "-o", type=str, default="Screenshots", help="Output directory path")
    parser.add_argument(
        "--mode", "-m", type=str, default=None,
        choices=["Orbit", "TacticalOverview", "FirstPerson", "DroneInspect", "Isometric", "AgentFocus"],
        help="AdaptiveCamera mode override"
    )
    parser.add_argument("--unity-path", type=str, default=None, help="Custom path to Unity.exe")

    args = parser.parse_args()

    project_root = Path(__file__).resolve().parent.parent
    unity_bin = find_unity_binary(args.unity_path)

    if not unity_bin:
        print("[ERROR] Unity executable not found. Set UNITY_PATH or provide --unity-path.")
        sys.exit(1)

    scene_target = "all" if args.all else args.scene
    out_dir = project_root / args.out
    out_dir.mkdir(parents=True, exist_ok=True)
    log_file = project_root / "capture.log"

    print("==================================================================")
    print("  ⬡ H.I.V.E. // HEADLESS SCREENSHOT CAPTURE")
    print("==================================================================")
    print(f"Target Scene:     {scene_target}")
    print(f"Resolution:       {args.res}")
    print(f"Output Directory: {out_dir}")
    if args.mode:
        print(f"Camera Mode:      {args.mode}")
    print(f"Unity Binary:     {unity_bin}")
    print("------------------------------------------------------------------")

    cmd = [
        unity_bin,
        "-batchmode",
        "-quit",
        "-projectPath", str(project_root),
        "-executeMethod", "HIVE.Editor.HiveScreenshotCapture.CaptureCLI",
        "-scene", scene_target,
        "-res", args.res,
        "-out", str(out_dir),
        "-logFile", str(log_file)
    ]

    if args.mode:
        cmd.extend(["-cameraMode", args.mode])

    print("Dispatching Unity batchmode rendering...")
    try:
        proc = subprocess.run(cmd, check=False)
        if proc.returncode != 0:
            print(f"[FAILED] Unity exited with code {proc.returncode}.")
            if log_file.exists():
                print("Tail of capture.log:")
                with open(log_file, "r", encoding="utf-8", errors="ignore") as f:
                    lines = f.readlines()
                    print("".join(lines[-30:]))
            sys.exit(proc.returncode)

        print(f"[SUCCESS] Screen capture completed successfully. Files saved to {out_dir}")
        # List output files
        files = list(out_dir.glob("*.png"))
        print(f"Captured {len(files)} screenshot(s):")
        for f in files:
            size_kb = f.stat().st_size / 1024
            print(f"  ⬡ {f.name} ({size_kb:.1f} KB)")
        sys.exit(0)
    except Exception as e:
        print(f"[ERROR] Exception during execution: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
