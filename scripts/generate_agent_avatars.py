import math
import os
from PIL import Image, ImageDraw, ImageFilter, ImageChops

OUT_DIR = os.path.join(os.getcwd(), "public", "avatars")
os.makedirs(OUT_DIR, exist_ok=True)

# Super-sampled canvas (1024x1024 -> downsampled to 512x512 with LANCZOS)
SS = 1024
FINAL = 512

# Signature H.I.V.E. Palette: Pure OLED Black + #85D94B Phosphor Green
BLACK = (0, 0, 0, 255)
CARBON_DARK = (9, 11, 9, 255)
CARBON_MID = (16, 20, 16, 255)
CARBON_LIGHT = (26, 33, 26, 255)
GREEN = (133, 217, 75, 255)       # #85D94B
GREEN_DIM = (68, 118, 36, 255)
GREEN_CORE = (215, 255, 185, 255) # Hot phosphor center


def regular_polygon(cx, cy, r, sides, angle_deg=-90):
    pts = []
    for i in range(sides):
        a = math.radians(angle_deg + i * (360.0 / sides))
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


def draw_cyber_bust_base(base_draw, glow_draw, style="angular"):
    """Draws the dark carbon metallic head/shoulders silhouette and subtle rim lines."""
    # Subtle background hex grid rings
    for r in (440, 360):
        pts = regular_polygon(512, 512, r, 6, angle_deg=30)
        base_draw.polygon(pts, outline=(18, 26, 16, 255), width=2)

    # Shoulders / Collar Chassis
    shoulder_pts = [
        (140, 1024),
        (210, 770),
        (360, 700),
        (664, 700),
        (814, 770),
        (884, 1024),
    ]
    base_draw.polygon(shoulder_pts, fill=CARBON_DARK, outline=CARBON_LIGHT, width=4)

    # Inner chest armor plate
    chest_pts = [
        (310, 1024),
        (360, 740),
        (512, 790),
        (664, 740),
        (714, 1024),
    ]
    base_draw.polygon(chest_pts, fill=CARBON_MID, outline=GREEN_DIM, width=3)

    # Neck column with mechanical ribs
    base_draw.rectangle([432, 600, 592, 745], fill=CARBON_DARK, outline=CARBON_LIGHT, width=3)
    for y in (635, 670, 705):
        base_draw.line([(448, y), (576, y)], fill=GREEN_DIM, width=3)

    # Head silhouette based on style
    if style == "hex":
        head_pts = [
            (512, 160),
            (690, 255),
            (700, 470),
            (605, 635),
            (512, 665),
            (419, 635),
            (324, 470),
            (334, 255),
        ]
    elif style == "sleek":
        head_pts = [
            (512, 165),
            (665, 235),
            (680, 445),
            (590, 625),
            (512, 660),
            (434, 625),
            (344, 445),
            (359, 235),
        ]
    elif style == "armored":
        head_pts = [
            (512, 150),
            (705, 240),
            (725, 465),
            (635, 640),
            (512, 675),
            (389, 640),
            (299, 465),
            (319, 240),
        ]
    else:
        head_pts = [
            (512, 160),
            (680, 245),
            (695, 460),
            (605, 635),
            (512, 665),
            (419, 635),
            (329, 460),
            (344, 245),
        ]

    base_draw.polygon(head_pts, fill=CARBON_MID, outline=CARBON_LIGHT, width=5)

    # Inner facial chamfer
    inner_pts = [(512 + (x - 512) * 0.86, 415 + (y - 415) * 0.86) for (x, y) in head_pts]
    base_draw.polygon(inner_pts, fill=CARBON_DARK, outline=GREEN_DIM, width=3)

    # Glowing jawline & shoulder rim accents
    glow_draw.line([head_pts[2], head_pts[3], head_pts[4], head_pts[5], head_pts[6]], fill=GREEN, width=5)
    glow_draw.line([(210, 770), (360, 700), (512, 755), (664, 700), (814, 770)], fill=GREEN, width=5)


def draw_agent_features(codename, base_draw, glow_draw, core_draw):
    if codename == "PRIME":
        # Sovereign Command Crown + Hexagonal Tactical Visor
        draw_cyber_bust_base(base_draw, glow_draw, "hex")
        # Command Crown
        crown = [(410, 240), (460, 130), (512, 185), (564, 130), (614, 240)]
        glow_draw.line(crown, fill=GREEN, width=8)
        core_draw.line(crown, fill=GREEN_CORE, width=3)
        # Hex emblem on forehead
        hex_pts = regular_polygon(512, 265, 42, 6, 30)
        glow_draw.polygon(hex_pts, outline=GREEN, width=6)
        core_draw.polygon(hex_pts, outline=GREEN_CORE, width=2)
        # Dual-chevron Command Visor
        visor = [(355, 385), (512, 435), (669, 385), (645, 455), (512, 495), (379, 455)]
        glow_draw.polygon(visor, fill=GREEN, outline=GREEN_CORE, width=4)
        core_draw.line([(385, 415), (512, 455), (639, 415)], fill=GREEN_CORE, width=6)
        # Collar Command Hex
        c_hex = regular_polygon(512, 875, 58, 6, 30)
        glow_draw.polygon(c_hex, outline=GREEN, width=7)
        core_draw.ellipse([496, 859, 528, 891], fill=GREEN_CORE)

    elif codename == "SAGE":
        # Research & Analysis: Triple Ocular Data-Lenses + Constellation Halo
        draw_cyber_bust_base(base_draw, glow_draw, "sleek")
        # Orbital Halo Arc
        glow_draw.arc([230, 110, 794, 674], start=200, end=340, fill=GREEN, width=6)
        for angle in (215, 245, 270, 295, 325):
            rad = math.radians(angle)
            nx = 512 + 282 * math.cos(rad)
            ny = 392 + 282 * math.sin(rad)
            glow_draw.ellipse([nx - 12, ny - 12, nx + 12, ny + 12], fill=GREEN)
            core_draw.ellipse([nx - 5, ny - 5, nx + 5, ny + 5], fill=GREEN_CORE)
        # Left & Right Ocular Lenses + Third Eye Synthesis Lens
        for (cx, cy, r) in [(425, 425, 52), (599, 425, 52), (512, 310, 36)]:
            glow_draw.ellipse([cx - r, cy - r, cx + r, cy + r], outline=GREEN, width=7)
            glow_draw.ellipse([cx - r * 0.55, cy - r * 0.55, cx + r * 0.55, cy + r * 0.55], fill=GREEN)
            core_draw.ellipse([cx - r * 0.25, cy - r * 0.25, cx + r * 0.25, cy + r * 0.25], fill=GREEN_CORE)
        glow_draw.line([(477, 425), (547, 425)], fill=GREEN, width=6)
        glow_draw.line([(512, 346), (512, 425)], fill=GREEN, width=5)

    elif codename == "FORGE":
        # Engineering & Build: Heavy Industrial Visor + Circuit Traces
        draw_cyber_bust_base(base_draw, glow_draw, "armored")
        # Industrial Slotted Visor Box
        glow_draw.rounded_rectangle([350, 365, 674, 475], radius=18, outline=GREEN, width=8)
        for y in (395, 420, 445):
            glow_draw.line([(375, y), (649, y)], fill=GREEN, width=7)
            core_draw.line([(395, y), (629, y)], fill=GREEN_CORE, width=3)
        # Forehead Circuit Bus
        for x in (450, 512, 574):
            glow_draw.line([(x, 210), (x, 345)], fill=GREEN, width=6)
            glow_draw.ellipse([x - 10, 200, x + 10, 220], fill=GREEN)
            core_draw.ellipse([x - 4, 206, x + 4, 214], fill=GREEN_CORE)
        # Respirator / Faceplate Grille
        for x in range(452, 580, 24):
            glow_draw.line([(x, 525), (x, 605)], fill=GREEN, width=6)

    elif codename == "SENTINEL":
        # Security & Audit: Tactical Aegis Shield Visor + Radar Lock
        draw_cyber_bust_base(base_draw, glow_draw, "armored")
        # Shield Crest Outline over Faceplate
        shield = [(360, 270), (664, 270), (650, 490), (512, 635), (374, 490)]
        glow_draw.polygon(shield, outline=GREEN, width=7)
        # Sharp Angular V-Slit Tactical Visor
        v_visor = [(365, 375), (512, 440), (659, 375), (659, 415), (512, 485), (365, 415)]
        glow_draw.polygon(v_visor, fill=GREEN)
        core_draw.line([(380, 395), (512, 460), (644, 395)], fill=GREEN_CORE, width=5)
        # Vertical Security Laser Line
        glow_draw.line([(512, 195), (512, 625)], fill=GREEN, width=6)
        core_draw.line([(512, 220), (512, 600)], fill=GREEN_CORE, width=2)

    elif codename == "VOICE":
        # Communications: Harmonic Soundwave Visor + Acoustic Resonance Rings
        draw_cyber_bust_base(base_draw, glow_draw, "sleek")
        # Sleek Curved Visor Frame
        glow_draw.rounded_rectangle([360, 370, 664, 455], radius=40, outline=GREEN, width=7)
        glow_draw.line([(390, 412), (634, 412)], fill=GREEN, width=8)
        core_draw.line([(410, 412), (614, 412)], fill=GREEN_CORE, width=4)
        # Acoustic Soundwave Equalizer on Vocal Mask
        bars = [28, 52, 84, 110, 70, 120, 84, 52, 28]
        start_x = 400
        for i, h in enumerate(bars):
            bx = start_x + i * 28
            glow_draw.line([(bx, 545 - h // 2), (bx, 545 + h // 2)], fill=GREEN, width=8)
            core_draw.line([(bx, 545 - h // 3), (bx, 545 + h // 3)], fill=GREEN_CORE, width=3)
        # Side Resonance Waves
        for r in (65, 95):
            glow_draw.arc([230 - r, 360, 350, 490], start=120, end=240, fill=GREEN, width=5)
            glow_draw.arc([674, 360, 794 + r, 490], start=300, end=60, fill=GREEN, width=5)

    elif codename == "ARCHIVIST":
        # Knowledge & Memory: Crystalline Neural Lattice & Knowledge Graph Crown
        draw_cyber_bust_base(base_draw, glow_draw, "hex")
        # Knowledge Graph Nodes & Edges above/on Forehead
        nodes = [(512, 215), (415, 275), (609, 275), (460, 345), (564, 345), (512, 290)]
        edges = [(0, 1), (0, 2), (0, 5), (1, 3), (2, 4), (5, 3), (5, 4), (3, 4), (1, 5), (2, 5)]
        for i, j in edges:
            glow_draw.line([nodes[i], nodes[j]], fill=GREEN, width=4)
        for nx, ny in nodes:
            glow_draw.ellipse([nx - 11, ny - 11, nx + 11, ny + 11], fill=GREEN)
            core_draw.ellipse([nx - 5, ny - 5, nx + 5, ny + 5], fill=GREEN_CORE)
        # Faceted Diamond Oculars
        for cx in (430, 594):
            d_pts = [(cx, 390), (cx + 55, 430), (cx, 470), (cx - 55, 430)]
            glow_draw.polygon(d_pts, fill=GREEN, outline=GREEN_CORE, width=3)
            core_draw.ellipse([cx - 10, 420, cx + 10, 440], fill=GREEN_CORE)

    elif codename == "CONDUCTOR":
        # Operations & Queue: Dual Synchronized Chronometer Rings & Parallel Routing Bus
        draw_cyber_bust_base(base_draw, glow_draw, "hex")
        # Parallel Routing Lines across Visor
        for idx, y in enumerate((385, 415, 445)):
            w = 8 if idx == 1 else 5
            glow_draw.line([(345, y), (679, y)], fill=GREEN, width=w)
            core_draw.line([(375, y), (649, y)], fill=GREEN_CORE, width=2)
        # Synchronized Timing Nodes on Visor
        for cx in (430, 512, 594):
            glow_draw.ellipse([cx - 26, 415 - 26, cx + 26, 415 + 26], fill=CARBON_DARK, outline=GREEN, width=7)
            core_draw.ellipse([cx - 9, 415 - 9, cx + 9, 415 + 9], fill=GREEN_CORE)
        # Forehead Queue Progress Chevron
        for y in (245, 285, 325):
            glow_draw.line([(455, y + 18), (512, y), (569, y + 18)], fill=GREEN, width=6)

    elif codename == "VISION":
        # Creative & Design: Geometric Prism Mask + Aperture Iris Lens
        draw_cyber_bust_base(base_draw, glow_draw, "sleek")
        # Low-poly Faceted Wireframe Lines across Face
        facets = [
            [(512, 180), (410, 310), (512, 355)],
            [(512, 180), (614, 310), (512, 355)],
            [(410, 310), (355, 465), (512, 540)],
            [(614, 310), (669, 465), (512, 540)],
        ]
        for tri in facets:
            glow_draw.polygon(tri, outline=GREEN, width=4)
        # Precision Camera Aperture Right Eye + Geometric Left Visor
        glow_draw.ellipse([535, 365, 655, 485], outline=GREEN, width=8)
        glow_draw.ellipse([568, 398, 622, 452], fill=GREEN)
        core_draw.ellipse([583, 413, 607, 437], fill=GREEN_CORE)
        glow_draw.polygon([(365, 410), (495, 410), (475, 445), (385, 445)], fill=GREEN)
        core_draw.line([(380, 427), (480, 427)], fill=GREEN_CORE, width=4)

    elif codename == "LEDGER":
        # Finance & Resources: Quantitative Matrix Visor & Equilibrium Scales Crest
        draw_cyber_bust_base(base_draw, glow_draw, "hex")
        # Equilibrium Balance Beam on Forehead
        glow_draw.line([(405, 280), (619, 280)], fill=GREEN, width=6)
        glow_draw.polygon([(512, 225), (538, 280), (486, 280)], fill=GREEN)
        glow_draw.ellipse([390, 265, 420, 295], fill=GREEN)
        glow_draw.ellipse([604, 265, 634, 295], fill=GREEN)
        # Quantitative Matrix Grid Visor
        glow_draw.rectangle([360, 375, 664, 465], outline=GREEN, width=7)
        for x in range(398, 660, 38):
            glow_draw.line([(x, 375), (x, 465)], fill=GREEN, width=3)
        glow_draw.line([(360, 420), (664, 420)], fill=GREEN, width=5)
        core_draw.line([(385, 420), (639, 420)], fill=GREEN_CORE, width=3)

    elif codename == "LINK":
        # Integration & Tools: Interlocking Hexagonal Bridge Nodes & Conduits
        draw_cyber_bust_base(base_draw, glow_draw, "hex")
        # Twin Interlocking Hexagonal Visor Lenses
        for cx in (445, 579):
            h_pts = regular_polygon(cx, 420, 66, 6, 0)
            glow_draw.polygon(h_pts, outline=GREEN, width=8)
            inner_h = regular_polygon(cx, 420, 32, 6, 0)
            glow_draw.polygon(inner_h, fill=GREEN)
            core_draw.ellipse([cx - 10, 410, cx + 10, 430], fill=GREEN_CORE)
        # External Conduit Bridges connecting to Temples
        glow_draw.line([(240, 420), (379, 420)], fill=GREEN, width=8)
        glow_draw.line([(645, 420), (784, 420)], fill=GREEN, width=8)
        core_draw.line([(260, 420), (375, 420)], fill=GREEN_CORE, width=3)
        core_draw.line([(649, 420), (764, 420)], fill=GREEN_CORE, width=3)

    elif codename == "GAUGE":
        # Quality & Verification: Target Reticle Monocle + Telemetry Verification HUD
        draw_cyber_bust_base(base_draw, glow_draw, "armored")
        # Left Eye Precision Target Reticle Monocle
        cx, cy = 430, 420
        glow_draw.ellipse([cx - 68, cy - 68, cx + 68, cy + 68], outline=GREEN, width=8)
        glow_draw.ellipse([cx - 32, cy - 32, cx + 32, cy + 32], outline=GREEN, width=5)
        glow_draw.line([(cx - 88, cy), (cx + 88, cy)], fill=GREEN, width=5)
        glow_draw.line([(cx, cy - 88), (cx, cy + 88)], fill=GREEN, width=5)
        core_draw.ellipse([cx - 10, cy - 10, cx + 10, cy + 10], fill=GREEN_CORE)
        # Right Eye Telemetry Bar
        glow_draw.rounded_rectangle([540, 395, 660, 445], radius=12, fill=GREEN)
        core_draw.line([(558, 420), (642, 420)], fill=GREEN_CORE, width=5)
        # Forehead Verification Check Emblem
        check_pts = [(475, 275), (502, 305), (558, 240)]
        glow_draw.line(check_pts, fill=GREEN, width=10)
        core_draw.line(check_pts, fill=GREEN_CORE, width=4)

    elif codename == "SPARK":
        # Innovation & Experiments: High-Voltage Plasma Arc Visor & Tesla Horns
        draw_cyber_bust_base(base_draw, glow_draw, "sleek")
        # Twin Tesla Energy Coils on Temples/Crown
        glow_draw.line([(330, 290), (275, 165)], fill=GREEN, width=10)
        glow_draw.line([(694, 290), (749, 165)], fill=GREEN, width=10)
        glow_draw.ellipse([255, 145, 295, 185], fill=GREEN)
        glow_draw.ellipse([729, 145, 769, 185], fill=GREEN)
        # Electric Lightning Visor
        bolt = [(350, 430), (435, 380), (485, 435), (545, 375), (590, 430), (674, 385)]
        glow_draw.line(bolt, fill=GREEN, width=14)
        core_draw.line(bolt, fill=GREEN_CORE, width=5)
        # Forehead Plasma Core Star
        star = regular_polygon(512, 265, 45, 4, 45)
        glow_draw.polygon(star, fill=GREEN)
        core_draw.ellipse([496, 249, 528, 281], fill=GREEN_CORE)

    elif codename == "HQ":
        # #agency-hq Collective Borg Emblem
        for r, w in [(430, 8), (320, 6), (200, 8)]:
            pts = regular_polygon(512, 512, r, 6, 30)
            glow_draw.polygon(pts, outline=GREEN, width=w)
        # 12 Agent Nodes around the ring
        for i in range(12):
            a = math.radians(i * 30 - 90)
            nx = 512 + 320 * math.cos(a)
            ny = 512 + 320 * math.sin(a)
            glow_draw.line([(512, 512), (nx, ny)], fill=GREEN_DIM, width=4)
            glow_draw.ellipse([nx - 22, ny - 22, nx + 22, ny + 22], fill=GREEN)
            core_draw.ellipse([nx - 9, ny - 9, nx + 9, ny + 9], fill=GREEN_CORE)
        center_hex = regular_polygon(512, 512, 95, 6, 30)
        glow_draw.polygon(center_hex, fill=GREEN)
        core_draw.polygon(regular_polygon(512, 512, 48, 6, 30), fill=GREEN_CORE)


def render_avatar(codename):
    base = Image.new("RGBA", (SS, SS), BLACK)
    glow = Image.new("RGBA", (SS, SS), (0, 0, 0, 0))
    core = Image.new("RGBA", (SS, SS), (0, 0, 0, 0))

    base_draw = ImageDraw.Draw(base)
    glow_draw = ImageDraw.Draw(glow)
    core_draw = ImageDraw.Draw(core)

    draw_agent_features(codename, base_draw, glow_draw, core_draw)

    # Outer frame border hex/circle accent
    glow_draw.rounded_rectangle([24, 24, SS - 24, SS - 24], radius=160, outline=GREEN_DIM, width=8)

    # Convert glow layer to RGB on black for true additive phosphor bloom
    glow_rgb = Image.new("RGB", (SS, SS), (0, 0, 0))
    glow_rgb.paste(glow, mask=glow.split()[3])

    bloom_ultra = glow_rgb.filter(ImageFilter.GaussianBlur(radius=56))
    bloom_wide = glow_rgb.filter(ImageFilter.GaussianBlur(radius=26))
    bloom_mid = glow_rgb.filter(ImageFilter.GaussianBlur(radius=10))

    comp_rgb = base.convert("RGB")
    comp_rgb = ImageChops.add(comp_rgb, bloom_ultra)
    comp_rgb = ImageChops.add(comp_rgb, bloom_wide)
    comp_rgb = ImageChops.add(comp_rgb, bloom_mid)

    comp = comp_rgb.convert("RGBA")
    comp = Image.alpha_composite(comp, glow)
    comp = Image.alpha_composite(comp, core)

    final_img = comp.resize((FINAL, FINAL), Image.Resampling.LANCZOS)
    out_path = os.path.join(OUT_DIR, f"{codename}.png")
    final_img.save(out_path, "PNG", optimize=True)
    return out_path


if __name__ == "__main__":
    agents = [
        "PRIME", "SAGE", "FORGE", "SENTINEL", "VOICE", "ARCHIVIST",
        "CONDUCTOR", "VISION", "LEDGER", "LINK", "GAUGE", "SPARK", "HQ"
    ]
    for code in agents:
        p = render_avatar(code)
        print(f"Generated {code}: {p} ({os.path.getsize(p)} bytes)")
