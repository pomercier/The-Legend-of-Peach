#!/usr/bin/env python3
"""Render v0.10.0 flagship galleries, Before/After (v0.9.0 vs v0.10.0) comparison plate,
720p real-SPC700-audio showcase video, and export updated 33-table Mega SQLite DB."""

import argparse
import datetime
import hashlib
import json
import shutil
import sqlite3
import subprocess
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ART = Path("/tmp/v0100_media")
QA = Path("/tmp/v0100_qa")
DB = Path("/tmp/legend_of_peach_mega_qa.sqlite")


def pick_frame(slug: str, name: str) -> Path:
    p = QA / slug / "frames" / name
    if p.exists():
        return p
    fs = sorted((QA / slug / "frames").glob("f_*.png"))
    return fs[min(len(fs) - 1, max(0, len(fs) // 2))]


def render_12_panel_gallery(out_paths: list[Path]):
    panels = [
        ("1. World 1 Yoshi's Island + SMW Gold HUD (ZRI-014)", pick_frame("v0100_01_warp_pipe_network_faust", "f_002.png")),
        ("2. Real Warp Pipe -> Toad Town Plaza (0x18, 260c)", pick_frame("v0100_01_warp_pipe_network_faust", "f_007.png")),
        ("3. Real Warp Pipe -> Vanilla Dome Mtn (0x03, MU-045)", pick_frame("v0100_01_warp_pipe_network_faust", "f_020.png")),
        ("4. Real Warp Pipe -> Lake Cheep-Cheep (0x35, 280c)", pick_frame("v0100_05_frog_suit_and_vanilla_dome", "f_013.png")),
        ("5. SMW Frog Suit Auto-Swim (Pal 10, MU-069)", pick_frame("v0100_05_frog_suit_and_vanilla_dome", "f_019.png")),
        ("6. Playable Luigi Mode + High Jump (Down+L+R, MU-068)", pick_frame("v0100_03_luigi_pswitch_sram", "f_006.png")),
        ("7. P-Switch 2x Gold Rush (250->272c, SRAM Saved)", pick_frame("v0100_03_luigi_pswitch_sram", "f_013.png")),
        ("8. Yoshi Tongue Swallow (Koopa Gulped, +1 Heart/+12c)", pick_frame("v0100_04_yoshi_tongue_swallow", "f_009.png")),
        ("9. SMW Dry Bones (0xA7) Crumble & GP Shatter (0x026)", pick_frame("v0100_06_dry_bones_crumble_shatter", "f_009.png")),
        ("10. Live Bestiary: Chain Chomp (0x6A) & Hammer Bros", pick_frame("v0100_08_bestiary_chomp_room0b0", "f_010.png")),
        ("11. Cycle-Accurate bsnes King Bowser Phase 2 (281c)", pick_frame("v0100_11_bsnes_final_bowser_phase2", "f_018.png")),
        ("12. Blank-SRAM Playthrough: Star-Torch & Rainy OW", pick_frame("v0100_12_clean_sram_playthrough", "f_014.png")),
    ]
    w, h = 512, 448
    cols, rows = 4, 3
    pad, hdr, cap = 10, 44, 24
    canvas = Image.new("RGB", (cols * (w + pad) + pad, rows * (h + cap + pad) + hdr + pad), (14, 18, 28))
    draw = ImageDraw.Draw(canvas)
    draw.text(
        (16, 14),
        "THE LEGEND OF PEACH v0.10.0 - SUPER MARIO ADVENTURE BEYOND LIMITS (LIVE DUAL-CORE SNES CAPTURES: faust + bsnes)",
        fill=(255, 224, 96),
    )
    for i, (label, fpath) in enumerate(panels):
        r, c = divmod(i, cols)
        x = pad + c * (w + pad)
        y = hdr + pad + r * (h + cap + pad)
        im = Image.open(fpath).convert("RGB").resize((w, h), Image.NEAREST)
        draw.rectangle([x - 2, y - 2, x + w + 1, y + h + cap + 1], outline=(70, 95, 140), width=2)
        draw.rectangle([x, y, x + w, y + cap], fill=(24, 32, 52))
        draw.text((x + 8, y + 6), label, fill=(235, 244, 255))
        canvas.paste(im, (x, y + cap))
    for op in out_paths:
        op.parent.mkdir(parents=True, exist_ok=True)
        canvas.save(op, optimize=True)
    print(f"Saved 12-panel gallery -> {out_paths[0]}")


def render_before_after_plate(out_paths: list[Path]):
    pairs = [
        (
            "BEFORE (v0.9.0): Vanilla Flat HUD & Stationary Pipe Bonus (255c)",
            pick_frame("v0100_13_v090_baseline_compare", "f_005.png"),
            "AFTER (v0.10.0): SMW Gold-Riveted HUD + Real Warp -> Toad Town (260c)",
            pick_frame("v0100_01_warp_pipe_network_faust", "f_007.png"),
        ),
        (
            "BEFORE (v0.9.0): Classic Mario Only on Yoshi's Island (250c)",
            pick_frame("v0100_13_v090_baseline_compare", "f_002.png"),
            "AFTER (v0.10.0): Playable Luigi + P-Switch 2x Gold Rush (272c, SRAM)",
            pick_frame("v0100_03_luigi_pswitch_sram", "f_013.png"),
        ),
        (
            "BEFORE (v0.9.0): 108/128 OW Screens & Unwarped World",
            pick_frame("v0100_13_v090_baseline_compare", "f_004.png"),
            "AFTER (v0.10.0): 128/128 OW + Vanilla Dome Terracotta Mtn (0x03)",
            pick_frame("v0100_02_warp_pipe_network_bsnes", "f_021.png"),
        ),
    ]
    w, h = 512, 448
    pad, hdr, cap = 12, 44, 24
    canvas = Image.new("RGB", (2 * w + 3 * pad, len(pairs) * (h + cap + pad) + hdr + pad), (14, 18, 28))
    draw = ImageDraw.Draw(canvas)
    draw.text(
        (16, 14),
        "THE LEGEND OF PEACH - REAL EMULATOR BEFORE (v0.9.0) VS AFTER (v0.10.0) COMPARISON",
        fill=(255, 224, 96),
    )
    for r, (bl, bf, al, af) in enumerate(pairs):
        y = hdr + pad + r * (h + cap + pad)
        for col, (lbl, fp, clr) in enumerate([(bl, bf, (255, 175, 120)), (al, af, (130, 255, 165))]):
            x = pad + col * (w + pad)
            im = Image.open(fp).convert("RGB").resize((w, h), Image.NEAREST)
            draw.rectangle([x - 2, y - 2, x + w + 1, y + h + cap + 1], outline=clr, width=2)
            draw.rectangle([x, y, x + w, y + cap], fill=(24, 32, 52))
            draw.text((x + 8, y + 6), lbl, fill=clr)
            canvas.paste(im, (x, y + cap))
    for op in out_paths:
        op.parent.mkdir(parents=True, exist_ok=True)
        canvas.save(op, optimize=True)
    print(f"Saved Before/After plate -> {out_paths[0]}")


def render_showcase_video(out_mp4: Path):
    clips = [
        ("v0100_01_warp_pipe_network_faust", 17.5, 34.5),   # 17s: 0x2C -> Toad Town -> Vanilla Dome!
        ("v0100_05_frog_suit_and_vanilla_dome", 31.0, 44.0),  # 13s: Vanilla Dome -> Lake Cheep-Cheep -> Frog Suit!
        ("v0100_03_luigi_pswitch_sram", 17.5, 28.5),        # 11s: Playable Luigi + High Jump + P-Switch 2x!
        ("v0100_04_yoshi_tongue_swallow", 17.5, 25.5),      # 8s: Yoshi Tongue Swallow!
        ("v0100_06_dry_bones_crumble_shatter", 14.0, 20.5), # 6.5s: Dry Bones!
        ("v0100_08_bestiary_chomp_room0b0", 14.0, 19.0),    # 5s: Chain Chomp & Hammer Bros!
        ("v0100_11_bsnes_final_bowser_phase2", 18.0, 29.5), # 11.5s: Cycle-accurate bsnes Final King Bowser Phase 2!
        ("v0100_12_clean_sram_playthrough", 55.0, 72.0),    # 17s: Blank-SRAM Star-Torch Chest + Rainy Overworld!
    ]
    seg_dir = QA / "video_segs"
    if seg_dir.exists():
        shutil.rmtree(seg_dir)
    seg_dir.mkdir(parents=True)
    concat_lines = []
    for i, (slug, t0, t1) in enumerate(clips):
        src = QA / slug / "raw.mov"
        if not src.exists():
            continue
        seg = seg_dir / f"seg_{i:02d}.mp4"
        subprocess.run(
            [
                "ffmpeg", "-y", "-v", "error",
                "-ss", str(t0), "-to", str(t1),
                "-i", str(src),
                "-vf", "scale=960:720:flags=neighbor,pad=1280:720:160:0:color=0x0E121C",
                "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
                "-c:a", "aac", "-b:a", "192k", "-ar", "44100", "-ac", "2",
                str(seg),
            ],
            check=True,
        )
        concat_lines.append(f"file '{seg.resolve()}'")
    list_file = seg_dir / "concat.txt"
    list_file.write_text("\n".join(concat_lines) + "\n")
    subprocess.run(
        [
            "ffmpeg", "-y", "-v", "error",
            "-f", "concat", "-safe", "0", "-i", str(list_file),
            "-c", "copy",
            str(out_mp4),
        ],
        check=True,
    )
    print(f"Saved 720p showcase MP4 ({out_mp4.stat().st_size // 1024} KiB) -> {out_mp4}")


def export_db_snapshot():
    con = sqlite3.connect(str(DB))
    tables = [r[0] for r in con.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").fetchall()]
    counts = {t: con.execute(f"SELECT COUNT(*) FROM \"{t}\"").fetchone()[0] for t in tables}
    total_rows = sum(counts.values())
    snap = {
        "db_path": str(DB),
        "exported_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "table_count": len(tables),
        "total_rows": total_rows,
        "tables": counts,
    }
    (ART / "scratch" / "mega_qa_db_v0100_summary.json").write_text(json.dumps(snap, indent=2))
    print(f"Mega SQLite DB: {len(tables)} tables, {total_rows} total rows")
    con.close()


if __name__ == "__main__":
    render_12_panel_gallery([
        ART / "legend_of_peach_v0.10.0_beyond_limits_gallery.png",
        ROOT / "docs/media/screenshots/25_v0100_beyond_limits_gallery.png",
    ])
    render_before_after_plate([
        ART / "legend_of_peach_v0.10.0_before_after_comparison.png",
        ROOT / "docs/media/screenshots/26_v0100_before_after_comparison.png",
    ])
    render_showcase_video(ART / "legend_of_peach_v0.10.0_beyond_limits_showcase.mp4")
    export_db_snapshot()
