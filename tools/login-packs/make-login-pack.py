#!/usr/bin/env python3
"""Builds login-screen packs (small MPQs) from patch-c.mpq and the client's own login Lua.

    make-login-pack.py --source patch-c.mpq --glue DIR --out OUTDIR [--shots DIR] [--logo logo.tga] NAME [NAME ...]

DIR holds the client's Interface/GlueXML files lgzg.lua, AccountLogin.lua and XlIlHI.lua (extract them from
Patch-I.mpq). A pack holds one screen's model, textures and music plus rewritten copies of those three files:
  lgzg.lua          scene list replaced by the one model; the old FTA frame art removed
  AccountLogin.lua  old clock/FPS frames and fixed widget positions removed
  XlIlHI.lua        + the themed UI (theme-ui.lua): side columns, panel, buttons, website link
and two side-column pictures (Interface\\LoginScreen\\<name>_l.tga / _r.tga) made from a clean screenshot of the
screen (--shots DIR with clean-<name>.png at 16:9), which also gives the theme colours. Installed as
Data\\patch-ZZ.mpq a pack overrides the client's login screen; deleting the file brings the default back.
"""
import argparse, colorsys, os, re, shutil, struct, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mpqlib as M
import blp

HERE = os.path.dirname(os.path.abspath(__file__))
SITE_URL = "https://github.com/Ghostsupthere/fta-linux-client"
FONT_DIR = os.path.join(HERE, "..", "..", "launcher", "assets", "fonts")   # the launcher's own fonts (SIL OFL)
FONT_FILES = ["CinzelBold.ttf", "CinzelBlack.ttf", "CinzelDecorative-Bold.ttf", "OFL-Cinzel.txt", "OFL-CinzelDecorative.txt"]

# which logo colour goes with which screen (logo-<colour>.png in --logos); change here to taste
LOGO_FOR = {"northrend": "blue", "cataclysm": "default", "pandaria": "gold", "warlords": "red", "legion": "green",
            "legion-alt": "green", "battleforazeroth": "earth", "shadowlands": "blueorange", "dragonflight": "gold",
            "warwithin": "default"}

THEMES = {
    "northrend":   dict(title="Wrath of the Lich King", model=r"Interface\Glues\Models\UI_MAINMENU_NORTHREND\UI_MainMenu_Northrend_noSindra.m2", music=r"Sound\Music\GlueScreenMusic\wotlk_main_title.mp3"),   # the client's own file, not packed
    "cataclysm":   dict(title="Cataclysm",              model=r"Interface\Glues\Models\ui_mainmenu_cataclysm\ui_mainmenu_cataclysm.m2",   music=r"Sound\music\cataclysm\mus_shattering_uu01.mp3"),
    "pandaria":    dict(title="Mists of Pandaria",      model=r"Interface\Glues\Models\ui_mainmenu_pandaria\ui_mainmenu_pandaria.m2",     music=r"Sound\music\pandaria\mus_50_heartofpandaria_01.mp3"),
    "warlords":    dict(title="Warlords of Draenor",    model=r"Interface\Glues\Models\ui_mainmenu_warlords\ui_mainmenu_warlords.m2",     music=r"Sound\music\draenor\mus_60_asiegeofworlds_maintitle.mp3"),
    "legion":      dict(title="Legion",                 model=r"Interface\Glues\Models\ui_mainmenu_legion\ui_mainmenu_legion.m2",         music=r"Sound\music\legion\mus_70_kingdomswillburn_maintitle.mp3"),
    "legion-alt":  dict(title="Legion (alternate)",     model=r"Interface\Glues\Models\ui_mainmenu_legio2\ui_mainmenu_legio2.m2",         music=r"Sound\music\legion\mus_70_kingdomswillburn_maintitle.mp3"),
    "battleforazeroth": dict(title="Battle for Azeroth", model=r"Interface\Glues\Models\ui_mainmenu_battleforazeroth\ui_mainmenu_battleforazeroth.m2", music=r"Sound\music\battleforazeroth\mus_80_beforethestorm_maintitle.mp3"),
    "shadowlands": dict(title="Shadowlands",            model=r"Interface\Glues\Models\ui_mainmenu_shadowlands\ui_mainmenu_shadowlands.m2", music=r"Sound\music\shadowlands\mus_90_throughtheroofoftheworld_maintitle.mp3"),
    "dragonflight": dict(title="Dragonflight",          model=r"Interface\Glues\Models\ui_mainmenu_dragonflight\ui_mainmenu_dragonisles.m2", music=r"Sound\music\dragonflight\Dragonflight_Login_Screen_Music.mp3"),
    "warwithin":   dict(title="The War Within",         model=r"Interface\Glues\models\ui_mainmenu_warwithin\ui_mainmenu_warwithin.m2",  music=r"Sound\music\warwithin\mus_110_thewarwithin_maintitle.mp3"),
}


def norm(p):
    return p.replace("/", "\\")


def textures(data):
    n, o = struct.unpack_from("<II", data, 0x50)
    out = []
    for i in range(n):
        t, fl, ln, ofs = struct.unpack_from("<IIII", data, o + i * 16)
        if t == 0 and ln > 1:
            out.append(norm(data[ofs:ofs + ln - 1].decode("latin1")))
    return out


# ------------------------------------------------------------------------------------------- Lua rewrites
def read(path, enc):
    return open(path, encoding=enc, newline="").read().replace("\r\n", "\n")


def lgzg_for(base, model, music, camera=0):
    src = read(base, "utf-8-sig")
    a = src.index("\t\t{1, 0.282, 0.788")
    b = src.index("\nend\n\nLOGIN_MODEL_LIGHTS")
    entry = '\t\t{1, 0, 0, 0, 0, 1, 1, _, 0, 1, 1, "%s", _, %d}\n\t\t}' % (model.replace("\\", "/"), camera)
    src = src[:a] + entry + src[b:]
    src = src.replace('{-1, "Interface/LoginScreen/Fondo.blp"}', "{-1, {0, 0, 0, 1}}", 1)
    # the model's own camera (index 0) only applies once the model is set
    src = src.replace("mod:SetModel(m[12])\n", "mod:SetModel(m[12])\n\t\t\t\t\t\t\t\tif m[14] then mod:SetCamera(m[14]) end\n", 1)
    # the old frame art (logo, side panels, bars) is replaced by the themed UI
    a = src.index("local LOGO_TEXTURE_PATH")
    b = src.index("function newScene()")
    src = src[:a] + src[b:]
    # the default screen's walking-dog scripts and lighting do not apply to a single environment model
    cut = src.index("-- Definición de tipos de modelo")
    src = src[:cut] + "\n"
    music_line = 'login_music_path = "%s"' % music.replace("\\", "/") if music else "login_music_path = false"
    return src.replace('login_music_path = "Interface/LoginScreen/Intro.mp3"', music_line, 1)


def account_login_for(base):
    src = read(base, "latin-1")
    cut = src.index("local ANIMATION_DURATION = 2.5")           # old clock + FPS frames and their animation
    src = src[:cut] + "\n"
    src = re.sub(r"function MyLoginAddon_OnLoad\(self\).*?\nend\n", "function MyLoginAddon_OnLoad(self)\nend\n", src, count=1, flags=re.S)
    for line in ('AccountLoginPasswordEdit:SetPoint("BOTTOM", -586, 450);', 'AccountLoginLoginButton:SetPoint("BOTTOM", -587, 340);'):
        src = src.replace(line, "")                               # fixed positions: the themed UI places these
    return src


# ------------------------------------------------------------------------------------------- pictures
def theme_from(shot):
    """Colours for the theme from a clean 16:9 screenshot (neutral ones without it)."""
    from PIL import Image
    neutral = dict(accent=(0.85, 0.72, 0.42), panel=(0.05, 0.05, 0.06, 0.84), text=(0.93, 0.92, 0.88))
    if not shot:
        return neutral
    im = Image.open(shot).convert("RGB").resize((192, 108))
    px = list(zip(im.getdata(), im.convert("HSV").getdata()))
    hist = [0.0] * 36
    for (r, g, b), (h, s, v) in px:
        s, v = s / 255, v / 255
        if s > 0.3 and v > 0.35:
            hist[int(h / 256 * 36) % 36] += s * v
    if max(hist) < 5:
        return neutral
    sm = [hist[i - 1] + 2 * hist[i] + hist[(i + 1) % 36] for i in range(36)]   # peak hue, smoothed
    accent = colorsys.hsv_to_rgb((sm.index(max(sm)) + 0.5) / 36, 0.62, 1.0)
    avg = [sum(p[0][i] for p in px) / len(px) / 255 for i in range(3)]
    panel = tuple(min(0.16, a * 0.22 + c * 0.05) for a, c in zip(avg, accent)) + (0.84,)
    text = tuple(0.92 * 0.9 + c * 0.1 for c in accent)
    return dict(accent=accent, panel=panel, text=text)


def logo_tga(png, out):
    """The logo as a 512 x 256 TGA (a power of two, as the client needs), cropped to its picture and centred."""
    from PIL import Image
    im = Image.open(png).convert("RGBA")
    im = im.crop(im.getchannel("A").point(lambda v: 255 if v > 8 else 0).getbbox())
    im.thumbnail((512, 256), Image.LANCZOS)
    canvas = Image.new("RGBA", (512, 256), (0, 0, 0, 0))
    canvas.alpha_composite(im, ((512 - im.width) // 2, (256 - im.height) // 2))
    canvas.save(out)


def eye_tgas(outdir):
    """The visibility icons: an open eye and a crossed-out one, white on clear (the UI tints them), 64 x 64 TGA."""
    from PIL import Image, ImageDraw
    S, K = 64, 8
    out = {}
    for name, crossed in (("eye", False), ("eye-off", True)):
        im = Image.new("RGBA", (S * K, S * K), (0, 0, 0, 0))
        d = ImageDraw.Draw(im)
        c = S * K / 2
        w, h = S * K * 0.46, S * K * 0.27
        pts = []
        for i in range(0, 101):                                   # almond outline: two arcs meeting at the corners
            x = -w + 2 * w * i / 100
            y = h * (1 - ((x / w) ** 2)) ** 0.9
            pts.append((c + x, c - y))
        for i in range(100, -1, -1):
            x = -w + 2 * w * i / 100
            y = h * (1 - ((x / w) ** 2)) ** 0.9
            pts.append((c + x, c + y))
        d.line(pts + [pts[0]], fill=(255, 255, 255, 255), width=int(S * K * 0.07), joint="curve")
        r = S * K * 0.15
        d.ellipse((c - r, c - r, c + r, c + r), fill=(255, 255, 255, 255))
        if crossed:
            m = S * K * 0.17
            d.line((m, S * K - m, S * K - m, m), fill=(0, 0, 0, 255), width=int(S * K * 0.15))      # a gap around the stroke
            d.line((m, S * K - m, S * K - m, m), fill=(255, 255, 255, 255), width=int(S * K * 0.065))
            # clear the gap again where the stroke's dark edge left the eye
        im = im.resize((S, S), Image.LANCZOS)
        if crossed:                                                # the dark gap becomes transparent
            px = im.load()
            for y in range(S):
                for x in range(S):
                    r_, g_, b_, a_ = px[x, y]
                    if r_ < 255 and a_ > 0:
                        px[x, y] = (255, 255, 255, int(a_ * r_ / 255))
        path = os.path.join(outdir, name + ".tga")
        im.save(path)
        out[name] = path
    im = Image.new("RGBA", (S * K, S * K), (0, 0, 0, 0))              # a circular arrow, clockwise (the rotate buttons)
    d = ImageDraw.Draw(im)
    import math
    c, R, wd = S * K / 2, S * K * 0.32, int(S * K * 0.10)
    d.arc((c - R, c - R, c + R, c + R), 20, 285, fill=(255, 255, 255, 255), width=wd)
    th = math.radians(285)
    P = (c + R * math.cos(th), c + R * math.sin(th))
    t, n, h, w = (-math.sin(th), math.cos(th)), (math.cos(th), math.sin(th)), S * K * 0.20, S * K * 0.17
    d.polygon([(P[0] + t[0] * h, P[1] + t[1] * h), (P[0] + n[0] * w, P[1] + n[1] * w), (P[0] - n[0] * w, P[1] - n[1] * w)], fill=(255, 255, 255, 255))
    path = os.path.join(outdir, "rotate.tga")
    im.resize((S, S), Image.LANCZOS).save(path)
    out["rotate"] = path
    im = Image.new("RGBA", (S * K, S * K), (0, 0, 0, 0))              # a chevron pointing right (the page buttons)
    d = ImageDraw.Draw(im)
    pts = [(S * K * 0.36, S * K * 0.2), (S * K * 0.66, S * K * 0.5), (S * K * 0.36, S * K * 0.8)]
    d.line(pts, fill=(255, 255, 255, 255), width=int(S * K * 0.12), joint="curve")
    for x, y in (pts[0], pts[2]):
        r = S * K * 0.06
        d.ellipse((x - r, y - r, x + r, y + r), fill=(255, 255, 255, 255))
    path = os.path.join(outdir, "chevron.tga")
    im.resize((S, S), Image.LANCZOS).save(path)
    out["chevron"] = path
    return out


def side_pictures(shot, outdir, name):
    """Left and right column pictures: the scene's own edges, mirrored outwards and blurred more the further out."""
    import numpy as np
    from PIL import Image, ImageFilter
    im = Image.open(shot).convert("RGB")
    W, H = im.size
    sw = int(W * 0.22)
    out = {}
    for side, box in (("l", (0, 0, sw, H)), ("r", (W - sw, 0, W, H))):
        strip = im.crop(box).transpose(Image.FLIP_LEFT_RIGHT)     # its inner edge is the scene's own edge
        mild = np.asarray(strip.filter(ImageFilter.GaussianBlur(5)), np.float32)
        heavy = np.asarray(strip.filter(ImageFilter.GaussianBlur(46)), np.float32)
        x = np.linspace(0, 1, sw, dtype=np.float32)
        d = 1 - x if side == "l" else x                           # distance from the seam: left picture has it at its right end
        k = (np.clip(d, 0, 1) ** 0.7)[None, :, None]
        a = (mild * (1 - k) + heavy * k) * (1 - 0.42 * k)         # blurrier and darker further out
        pic = Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).resize((256, 512), Image.LANCZOS).convert("RGBA")
        path = os.path.join(outdir, "%s_%s.tga" % (name, side))
        pic.save(path)                                            # uncompressed 32-bit TGA
        out[side] = path
    return out


# ------------------------------------------------------------------------------------------- Lua theme
def lua_color(c):
    return "{%s}" % ", ".join("%.3f" % v for v in c)


def theme_lua(name, colors, sides, logo):
    white = '"Interface\\\\Buttons\\\\WHITE8X8"'
    pic = lambda s: '"Interface\\\\LoginScreen\\\\%s_%s.tga"' % (name, s) if sides else white
    return ('{ id = "%s", accent = %s, panel = %s, text = %s,\n  sideLeft = %s, sideRight = %s, logo = %s,\n  siteUrl = "%s" }' % (
        name, lua_color(colors["accent"]), lua_color(colors["panel"]), lua_color(colors["text"]), pic("l"), pic("r"),
        '"Interface\\\\LoginScreen\\\\logo.tga"' if logo else "nil", SITE_URL))


DEBUG_SHOT = """
local _cr, _ha, _fk = false, false, false
local _t, _f, _demo, _cs, _dd = 0, CreateFrame("Frame", nil, GlueParent)
_f:SetScript("OnUpdate", function(self, e)
  _t = _t + e
  if %(demo)s and _t > 8 and not _demo then
    _demo = true
    AccountLoginAccountEdit:SetText("MyAccountName"); AccountLoginPasswordEdit:SetText("secret123")
    AccountLoginSaveAccountName:Click()
    AccountLoginSavePassword:Click()
  end
  if %(dump)s and _t > 20 and not _dd then
    _dd = true
    local dbg = CreateFrame("Frame", nil, GlueParent); dbg:SetFrameStrata("TOOLTIP")
    local lines = {}
    for i, c in ipairs(columns) do
      lines[#lines + 1] = "col" .. i .. " shown=" .. tostring(c:IsShown()) .. " w=" .. tostring(c:GetWidth()) .. " h=" .. tostring(c:GetHeight()) .. " strata=" .. tostring(c:GetFrameStrata()) .. " left=" .. tostring(c:GetLeft()) .. " alpha=" .. tostring(c:GetEffectiveAlpha()) .. " dimshown=" .. tostring(c.dim:IsShown())
    end
    lines[#lines + 1] = "screen=" .. tostring(GetCurrentGlueScreenName()) .. " GPleft=" .. tostring(GlueParent:GetLeft()) .. " GPw=" .. tostring(GlueParent:GetWidth())
    local f = dbg:CreateFontString(nil, "OVERLAY", "GlueFontNormalLarge"); f:SetPoint("CENTER", GlueParent, "CENTER", 0, 0)
    f:SetText("")
  end
  if %(fake)s and not _fk then
    _fk = true
    local L = { {"Gluesniffer","Human","Warlock",24,"Westfall"}, {"Magicalslut","Orc","Mage",25,"Orgrimmar"}, {"Arthas","Human","Paladin",26,"Duskwood"}, {"Mrfeeney","Dwarf","Shaman",25,"Ironforge"}, {"Finnaswipe","Night Elf","Druid",25,"Darnassus"} }
    GetNumCharacters = function() return #L end
    GetCharacterInfo = function(i) local c = L[i]; if c then return c[1], c[2], c[3], c[4], c[5], 0, nil, 0, 0, 0 end end
  end
  if %(charsel)s and _t > 12 and not _cs then _cs = true; SetGlueScreen("charselect") end
  if %(charsel)s and _t > 18 and not _cr then _cr = true; pcall(function() CharSelectCreateCharacterButton:Show() end) end
  if %(hideall)s and _t > 24 and not _ha then _ha = true; pcall(function() DesenfoBoton:Click() end) end
  if %(clean)s and _t > %(at)d - 3 then
    if panel then panel:Hide() end
    for _, b in ipairs(buttons) do b:Hide() end
    for _, c in ipairs(columns) do c:Hide() end
    if AccountLoginUI then AccountLoginUI:Hide() end
  end
  if _t > %(at)d then self:SetScript("OnUpdate", nil); Screenshot() end
end)
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source", required=True)
    ap.add_argument("--glue", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--shots", help="folder with clean-<name>.png screenshots (16:9) for colours and side columns")
    ap.add_argument("--fonts", default=FONT_DIR, help="folder with the Cinzel fonts (default: the launcher's assets/fonts)")
    ap.add_argument("--logo-color", help="use this logo colour for every named screen (overrides LOGO_FOR), e.g. red")
    ap.add_argument("--logos", help="folder with logo-<colour>.png files (see LOGO_FOR); shown at the top of the panel")
    ap.add_argument("--camera", type=int, default=0)
    ap.add_argument("--debug-shot", type=int, default=0, help="test builds only: take a screenshot after N seconds")
    ap.add_argument("--demo", action="store_true", help="with --debug-shot: type sample text and tick Remember name (to see them)")
    ap.add_argument("--charsel", action="store_true", help="with --debug-shot: switch to the character select screen first")
    ap.add_argument("--fake", action="store_true", help="with --charsel: pretend the account has five characters")
    ap.add_argument("--hideall", action="store_true", help="with --charsel: press the 'hide all' eye")
    ap.add_argument("--dump", action="store_true", help="with --charsel: list the character select regions on screen")
    ap.add_argument("--clean", action="store_true", help="with --debug-shot: hide the UI first (for column pictures)")
    ap.add_argument("names", nargs="+")
    a = ap.parse_args()
    pc = M.open_(a.source)
    os.makedirs(a.out, exist_ok=True)
    for name in a.names:
        t = THEMES[name]
        tmp = tempfile.mkdtemp()
        files = {}  # archived name -> local path

        def grab(arch):
            loc = os.path.join(tmp, str(len(files)))
            if not M.extract(pc, arch, loc):
                return False
            files[arch] = loc
            return True

        assert grab(t["model"]), "model not in source"
        data = open(files[t["model"]], "rb").read()
        base, stem = t["model"].rsplit("\\", 1)[0], t["model"].rsplit("\\", 1)[1][:-3]
        for skin in ("00", "01", "02", "03"):
            grab(base + "\\" + stem + skin + ".skin")
        miss = [tx for tx in textures(data) if tx not in files and not grab(tx)]
        converted = {}
        for tx in list(miss):                       # the model names a .blp that patch-c only has as .tga: convert it
            loc = os.path.join(tmp, "tga%d.tga" % len(converted))
            if tx.lower().endswith(".blp") and M.extract(pc, tx[:-4] + ".tga", loc):
                blp.to_blp(loc, loc + ".blp")
                converted[tx] = loc + ".blp"
                miss.remove(tx)
        files.update(converted)
        if t["music"]:
            grab(t["music"])

        shot = os.path.join(a.shots, "clean-%s.png" % name) if a.shots else None
        shot = shot if shot and os.path.exists(shot) else None
        colors = theme_from(shot)
        extra = {}
        if shot:
            sides = side_pictures(shot, tmp, name)
            extra[r"Interface\LoginScreen\%s_l.tga" % name] = sides["l"]
            extra[r"Interface\LoginScreen\%s_r.tga" % name] = sides["r"]
        logo = None
        if a.logos:
            logo = os.path.join(a.logos, "logo-%s.png" % (a.logo_color or LOGO_FOR[name]))
            logo_out = os.path.join(tmp, "logo.tga")
            logo_tga(logo, logo_out)
            extra[r"Interface\LoginScreen\logo.tga"] = logo_out
            colors["accent"] = theme_from(logo)["accent"]          # buttons and borders follow the logo's colour
        for n_, pth in eye_tgas(tmp).items():
            extra["Interface\\LoginScreen\\" + n_ + ".tga"] = pth
        for f in FONT_FILES:
            assert os.path.exists(os.path.join(a.fonts, f)), "font file missing: " + os.path.join(a.fonts, f)
            extra["Interface\\LoginScreen\\" + f] = os.path.join(a.fonts, f)

        ui = open(os.path.join(HERE, "theme-ui.lua"), encoding="utf-8").read()
        ui = ui.replace("__THEME__", theme_lua(name, colors, bool(shot), bool(logo)))
        ui = ui.replace("__DEBUG__", (DEBUG_SHOT % dict(clean="true" if a.clean else "false", at=a.debug_shot, demo="true" if a.demo else "false", charsel="true" if a.charsel else "false", dump="true" if a.dump else "false", hideall="true" if a.hideall else "false", fake="true" if a.fake else "false")) if a.debug_shot else "")
        lua = {
            r"Interface\GlueXML\lgzg.lua": (lgzg_for(os.path.join(a.glue, "lgzg.lua"), t["model"], t["music"], a.camera), "utf-8"),
            r"Interface\GlueXML\AccountLogin.lua": (account_login_for(os.path.join(a.glue, "AccountLogin.lua")), "latin-1"),
            r"Interface\GlueXML\XlIlHI.lua": (read(os.path.join(a.glue, "XlIlHI.lua"), "latin-1") + "\n" + ui, "latin-1"),
            # the old character-select overlay (Phoenix watermark, header art, blue/ember row borders): not used
            r"Interface\GlueXML\worktcharactertheme.lua": ("-- removed by the login-screen pack: the themed character select replaces this overlay\n", "latin-1"),
        }
        for arch, (text, enc) in lua.items():
            p = os.path.join(tmp, "lua-" + arch.rsplit("\\", 1)[1])
            open(p, "w", encoding=enc, newline="\n").write(text)
            extra[arch] = p

        out = os.path.join(a.out, "login-%s.mpq" % name)
        h = M.create(out)
        for arch, loc in {**files, **extra}.items():
            M.add(h, loc, arch, compress=not arch.lower().endswith((".mp3", ".tga")))
        M.close(h)
        shutil.rmtree(tmp)
        print("%s: %.1f MB, colours %s, side columns %s, missing textures: %s" % (
            out, os.path.getsize(out) / 1048576, "from screenshot" if shot else "neutral", "yes" if shot else "no", miss or "none"))


main()
