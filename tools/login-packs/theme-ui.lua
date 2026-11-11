
-----------------------------------------------------------------------------------------------------------
-- Themed login UI (added by the login-screen pack builder; the theme table below is generated per pack).
-- Everything is wrapped in pcall: if anything here fails the stock widgets still work and login is possible.
-----------------------------------------------------------------------------------------------------------
local T = __THEME__

local WHITE  = "Interface\\Buttons\\WHITE8X8"
local BORDER = "Interface\\Tooltips\\UI-Tooltip-Border"
local A, B, C = T.accent[1], T.accent[2], T.accent[3]

-- fonts of the launcher (Cinzel), shipped inside the pack; the stock font is used if one cannot be loaded
local FD = "Interface\\LoginScreen\\"
local FONT_HEAD, FONT_BLACK, FONT_TITLE = FD .. "CinzelBold.ttf", FD .. "CinzelBlack.ttf", FD .. "CinzelDecorative-Bold.ttf"
local FONT_INPUT = "Fonts\\ARIALN.TTF"
local function SetF(fs, path, size, flags)
    local ok = pcall(function() if not fs:SetFont(path, size, flags or "") then fs:SetFont("Fonts\\FRIZQT__.TTF", size, flags or "") end end)
    return ok
end

local function safe(fn, ...) local ok, err = pcall(fn, ...) return ok end

-- Opening the website goes to the client's repository
function AccountLogin_LaunchCommunitySite()
    PlaySound("gsLoginNewAccount")
    LaunchURL(T.siteUrl)
end

-- ---------------------------------------------------------------------------------------------------
-- 1. Side columns: fill whatever is left and right of the 16:9 login space so no screen area stays unrendered.
--    The columns are as wide as the space GlueParent leaves at each side and follow window changes.
-- ---------------------------------------------------------------------------------------------------
local columns = {}
local function MakeColumn(isLeft)
    local f = CreateFrame("Frame", isLeft and "RealmgateSideLeft" or "RealmgateSideRight", GlueParent)
    f:SetFrameStrata("FULLSCREEN_DIALOG")      -- the character select scene covers lower layers; the columns lie outside any dialog
    local tex = f:CreateTexture(nil, "BACKGROUND")
    tex:SetAllPoints(f)
    tex:SetTexture(isLeft and T.sideLeft or T.sideRight)
    -- a thin glow in the theme colour where the column meets the scene
    local seam = f:CreateTexture(nil, "ARTWORK")
    seam:SetTexture(WHITE)
    seam:SetWidth(44)
    if isLeft then
        seam:SetPoint("TOPRIGHT", f, "TOPRIGHT", 0, 0); seam:SetPoint("BOTTOMRIGHT", f, "BOTTOMRIGHT", 0, 0)
        seam:SetGradientAlpha("HORIZONTAL", A, B, C, 0, A, B, C, 0.12)
    else
        seam:SetPoint("TOPLEFT", f, "TOPLEFT", 0, 0); seam:SetPoint("BOTTOMLEFT", f, "BOTTOMLEFT", 0, 0)
        seam:SetGradientAlpha("HORIZONTAL", A, B, C, 0.12, A, B, C, 0)
    end
    local dim = f:CreateTexture(nil, "OVERLAY")                 -- the other glue screens get darker columns
    dim:SetTexture(0, 0, 0, 0.6)
    dim:SetAllPoints(f)
    dim:Hide()
    f.dim = dim
    f.isLeft = isLeft
    columns[#columns + 1] = f
    return f
end

local function LayoutColumns()
    local gl, gr = GlueParent:GetLeft(), GlueParent:GetRight()
    if not gl or not gr then return end
    local side = math.max(0, gl)                -- the space left of the 16:9 area (the area is centred)
    for _, f in ipairs(columns) do
        f:ClearAllPoints()
        if side < 0.5 then
            f:Hide()
        else
            f:SetWidth(side + 12)               -- reaches past the window edge, which is clipped
            if f.isLeft then
                f:SetPoint("TOPRIGHT", GlueParent, "TOPLEFT", 1, 30)
                f:SetPoint("BOTTOMRIGHT", GlueParent, "BOTTOMLEFT", 1, -30)
            else
                f:SetPoint("TOPLEFT", GlueParent, "TOPRIGHT", -1, 30)
                f:SetPoint("BOTTOMLEFT", GlueParent, "BOTTOMRIGHT", -1, -30)
            end
            f:Show()
        end
    end
end

safe(function()
    MakeColumn(true); MakeColumn(false)
    LayoutColumns()
end)

-- ---------------------------------------------------------------------------------------------------
-- 2. Panel and themed buttons
-- ---------------------------------------------------------------------------------------------------
local panel
local function Flat(f, bg, edge, edgeSize)
    f:SetBackdrop({ bgFile = WHITE, edgeFile = BORDER, tile = false, edgeSize = edgeSize or 14,
                    insets = { left = 3, right = 3, top = 3, bottom = 3 } })
    f:SetBackdropColor(bg[1], bg[2], bg[3], bg[4])
    f:SetBackdropBorderColor(edge[1], edge[2], edge[3], edge[4])
end

local buttons = {}
local function MakeButton(text, w, h, big, onClick, parent)
    local b = CreateFrame("Button", nil, parent or AccountLogin or GlueParent)
    b:SetFrameStrata("HIGH")
    b:SetSize(w, h)
    local base  = big and { A * 0.38, B * 0.38, C * 0.38, 0.92 } or { 0.05, 0.06, 0.07, 0.78 }
    local hover = big and { A * 0.62, B * 0.62, C * 0.62, 0.98 } or { A * 0.30, B * 0.30, C * 0.30, 0.92 }
    local edge  = { A, B, C, big and 1 or 0.7 }
    Flat(b, base, edge, 14)
    local fs = b:CreateFontString(nil, "OVERLAY", big and "GlueFontNormalLarge" or "GlueFontNormal")
    fs:SetPoint("CENTER", b, "CENTER", 0, 1)
    SetF(fs, big and FONT_BLACK or FONT_HEAD, big and 17 or 12)
    fs:SetText(text)
    fs:SetTextColor(T.text[1], T.text[2], T.text[3])
    b.label = fs
    b:SetScript("OnEnter", function(self) if self.on then self:SetBackdropColor(hover[1], hover[2], hover[3], hover[4]) end end)
    b:SetScript("OnLeave", function(self) self:SetBackdropColor(base[1], base[2], base[3], base[4]) end)
    b:SetScript("OnMouseDown", function(self) if self.on then fs:SetPoint("CENTER", self, "CENTER", 0, -1) end end)
    b:SetScript("OnMouseUp", function(self) fs:SetPoint("CENTER", self, "CENTER", 0, 1) end)
    b:SetScript("OnClick", function(self)
        if self.on then PlaySound("gsLoginNewAccount") onClick() end
    end)
    b.on = true
    buttons[#buttons + 1] = b
    return b
end

safe(function()
    panel = CreateFrame("Frame", "RealmgatePanel", AccountLogin or GlueParent)
    panel:SetFrameStrata("MEDIUM")
    local top = T.logo and 140 or 0                -- room for the logo above the fields
    panel:SetSize(330, 384 + top)
    panel:SetPoint("LEFT", GlueParent, "LEFT", 54, -6)
    Flat(panel, T.panel, { A, B, C, 0.85 }, 16)

    -- accent bar on top and a title slot (a logo picture can take its place later)
    local bar = panel:CreateTexture(nil, "ARTWORK")
    bar:SetTexture(WHITE)
    bar:SetPoint("TOPLEFT", panel, "TOPLEFT", 4, -4); bar:SetPoint("TOPRIGHT", panel, "TOPRIGHT", -4, -4)
    bar:SetHeight(3)
    bar:SetGradientAlpha("HORIZONTAL", A, B, C, 0, A, B, C, 1)

    if T.logo then
        local logo = panel:CreateTexture(nil, "ARTWORK")
        logo:SetTexture(T.logo)
        logo:SetSize(300, 150)
        logo:SetPoint("TOP", panel, "TOP", 0, -10)
    end
    local title = panel:CreateFontString(nil, "OVERLAY", "GlueFontNormalLarge")
    title:SetPoint("TOPLEFT", panel, "TOPLEFT", 28, -22)
    SetF(title, FONT_TITLE, 18)
    title:SetText(T.logo and "" or "SIGN IN")
    title:SetTextColor(A, B, C)

    -- the stock widgets keep doing the work; they are restyled and moved into the panel
    local function Field(edit, y)
        if not edit then return end
        edit:ClearAllPoints()
        edit:SetPoint("TOPLEFT", panel, "TOPLEFT", 28, y)
        edit:SetSize(274, 36)
        Flat(edit, { 0, 0, 0, 0.55 }, { A, B, C, 0.55 }, 12)
        edit:SetTextInsets(12, 10, 0, 2)
        edit:SetTextColor(1, 1, 1, 1)                  -- what is typed (or the password dots) is plain white
        safe(function()                                -- the stock edit font sets its own colour: give the box a white one
            if not RealmgateEditFont then
                local f = CreateFont("RealmgateEditFont")
                f:CopyFontObject(_G["GlueEditBoxFont"])
                f:SetTextColor(1, 1, 1, 1)
            end
            edit:SetFontObject(RealmgateEditFont)
            edit:SetTextColor(1, 1, 1, 1)
        end)
        local fill = _G[edit:GetName() .. "Fill"]
        for _, r in ipairs({ edit:GetRegions() }) do
            local ty = r.GetObjectType and r:GetObjectType()
            if ty == "Texture" then
                r:Hide()                                           -- the old paper background
            elseif ty == "FontString" and r ~= fill then
                r:SetTextColor(1, 1, 1, 1)                         -- the box's own text (black by default)
            end
        end
        if fill then
            fill:ClearAllPoints(); fill:SetPoint("LEFT", edit, "LEFT", 14, 1); fill:SetJustifyH("LEFT")
            SetF(fill, FONT_INPUT, 13)
        end
        safe(function()
            edit:HookScript("OnEditFocusLost", function(self) self:SetBackdropBorderColor(A, B, C, 0.55) end)
            edit:HookScript("OnEditFocusGained", function(self) self:SetBackdropBorderColor(A, B, C, 1) end)
        end)
    end
    Field(AccountLoginAccountEdit, -70 - top)
    Field(AccountLoginPasswordEdit, -116 - top)
    Field(AccountLoginTokenEdit, -162 - top)

    local function Check(box, text, y)
        if not box then return end
        box:ClearAllPoints()
        box:SetPoint("TOPLEFT", panel, "TOPLEFT", 26, y)
        box:SetSize(26, 26)
        if text then text:SetTextColor(T.text[1], T.text[2], T.text[3]); SetF(text, FONT_HEAD, 12) end
    end
    Check(AccountLoginSaveAccountName, AccountLoginSaveAccountNameText, -166 - top)
    Check(AccountLoginSavePassword, AccountLoginSavePasswordText, -190 - top)

    if AccountLoginDropDown then
        AccountLoginDropDown:ClearAllPoints()
        AccountLoginDropDown:SetPoint("TOPLEFT", panel, "TOPLEFT", 14, -46 - top)
    end

    local login = MakeButton("Log in", 274, 46, true, function() AccountLoginLoginButton:Click() end)
    login:SetPoint("TOPLEFT", panel, "TOPLEFT", 28, -228 - top)
    local web  = MakeButton("Website", 86, 30, false, function() AccountLogin_LaunchCommunitySite() end)
    web:SetPoint("TOPLEFT", panel, "TOPLEFT", 28, -296 - top)
    local opts = MakeButton("Options", 86, 30, false, function() OptionsButton:Click() end)
    opts:SetPoint("LEFT", web, "RIGHT", 8, 0)
    local quit = MakeButton("Quit", 86, 30, false, function() AccountLoginExitButton:Click() end)
    quit:SetPoint("LEFT", opts, "RIGHT", 8, 0)
    panel.login = login

    -- the stock buttons stay (they carry the logic) but are invisible and out of the way
    for _, name in ipairs({ "AccountLoginLoginButton", "AccountLoginCommunityButton", "OptionsButton", "AccountLoginExitButton" }) do
        local b = _G[name]
        if b then b:SetAlpha(0); b:EnableMouse(false) end
    end
    if NewsButton then NewsButton:Hide() end
end)


-- ---------------------------------------------------------------------------------------------------
-- 4. Character select: the pack's logo, themed buttons (the stock ones keep the logic), dim side columns.
--    Built once, the first time the screen's frames exist; everything hangs off CharacterSelect, so it
--    shows only there.
-- ---------------------------------------------------------------------------------------------------
local csBuilt, csButtons, selBox, csEyeAll, csEyeList, csLogo, hoverBox, csHover = false, {}, nil, nil, nil, nil, nil, nil
local EYE, EYE_OFF = FD .. "eye.tga", FD .. "eye-off.tga"
local function BuildCharSelect()
    csBuilt = true
    safe(function()
        if T.logo then
            local l = CharacterSelect:CreateTexture(nil, "OVERLAY")
            l:SetTexture(T.logo)
            l:SetSize(240, 120)
            l:SetPoint("TOPLEFT", GlueParent, "TOPLEFT", 14, -4)
            csLogo = l
        end
        local lp = CharacterSelectCharacterFrame
        if lp then Flat(lp, T.panel, { A, B, C, 0.8 }, 16) end

        local function Wrap(orig, text, w, h, big, place)
            if not orig then return end
            local b = MakeButton(text or (orig.GetText and orig:GetText()) or "", w, h, big, function() orig:Click() end, CharacterSelect)
            place(b)
            orig:SetAlpha(0); orig:EnableMouse(false)
            b.orig = orig
            csButtons[#csButtons + 1] = b
            return b
        end
        local function Hidable(b) if b then b.hideAll = true end end     -- gone in the "hide all" view, like the stock ones
        Wrap(CharSelectChangeRealmButton, "Change Realm", 190, 34, false, function(b) b:SetPoint("TOP", GlueParent, "TOP", 0, -14) end)
        local ew = Wrap(CharSelectEnterWorldButton, "Enter World", 240, 46, true, function(b) b:SetPoint("BOTTOM", GlueParent, "BOTTOM", 0, 40) end)
        Hidable(ew)

        Hidable(Wrap(CharacterSelectBackButton, "Back", 130, 32, false, function(b) b:SetPoint("BOTTOMLEFT", GlueParent, "BOTTOMLEFT", 22, 22) end))
        Hidable(Wrap(CharacterSelectAddonsButton, nil, 130, 32, false, function(b) b:SetPoint("BOTTOMLEFT", GlueParent, "BOTTOMLEFT", 22, 62) end))
        Wrap(OptionsButton2, "Options", 130, 32, false, function(b) b:SetPoint("BOTTOMLEFT", GlueParent, "BOTTOMLEFT", 22, 102) end)
        -- Create character, delete, and the page buttons (< >) above the list: the same look as the others
        local function Anchor(orig) return function(b) b:SetPoint("CENTER", orig, "CENTER", 0, 0) end end
        Hidable(Wrap(CharSelectCreateCharacterButton, nil, 196, 34, false, function(b) b:SetPoint("CENTER", lp or CharacterSelectCharacterFrame, "BOTTOM", 0, 32) end))
        Hidable(Wrap(CharacterSelectDeleteButton, "Delete", 110, 32, false, function(b) b:SetPoint("BOTTOMRIGHT", GlueParent, "BOTTOMRIGHT", -20, 50) end))
        for i, ob in ipairs(WorkTCharacterPageButtons or {}) do
            local b = Wrap(ob, "", 42, 28, false, Anchor(ob))
            if b then
                b.hideAll = true
                local ic = b:CreateTexture(nil, "OVERLAY")
                ic:SetSize(18, 18); ic:SetPoint("CENTER", b, "CENTER", 0, 0)
                ic:SetTexture(FD .. "chevron.tga")
                ic:SetVertexColor(T.text[1], T.text[2], T.text[3])
                if i == 1 then ic:SetTexCoord(1, 0, 0, 1) end         -- the left one points left
            end
        end
        -- the two visibility toggles: an eye for the whole interface, an eye with its label for the character list
        local function Eye(b, big)
            if not b then return end
            local ic = b:CreateTexture(nil, "OVERLAY")
            ic:SetSize(big and 22 or 20, big and 22 or 20)
            ic:SetTexture(EYE)
            ic:SetVertexColor(T.text[1], T.text[2], T.text[3])
            b.icon = ic
            return b
        end
        csEyeAll = Eye(Wrap(DesenfoBoton, "", 34, 32, false, function(b) b:SetPoint("BOTTOMLEFT", GlueParent, "BOTTOMLEFT", 160, 22) end))
        if csEyeAll then csEyeAll.icon:SetPoint("CENTER", csEyeAll, "CENTER", 0, 0) end
        csEyeList = Eye(Wrap(DesenfoBoton2, "War Band Character List", 262, 32, false, function(b) b:SetPoint("BOTTOMRIGHT", GlueParent, "BOTTOMRIGHT", -20, 12) end))
        if csEyeList then csEyeList.icon:SetPoint("RIGHT", csEyeList, "RIGHT", -10, 0) end
        -- the selected character row: the stock gold highlight is replaced by a themed box
        selBox = CreateFrame("Frame", nil, lp or CharacterSelect)
        selBox:SetFrameStrata("HIGH")
        selBox:SetSize(245, 64)
        Flat(selBox, { A * 0.6, B * 0.6, C * 0.6, 0.30 }, { A, B, C, 0.95 }, 12)
        selBox:Hide()
        hoverBox = CreateFrame("Frame", nil, lp or CharacterSelect)       -- a lighter box for the row under the mouse
        hoverBox:SetFrameStrata("HIGH")
        hoverBox:SetSize(245, 64)
        Flat(hoverBox, { A * 0.5, B * 0.5, C * 0.5, 0.10 }, { A, B, C, 0.45 }, 12)
        hoverBox:Hide()
        for i = 1, 8 do
            local row = _G["CharSelectCharacterButton" .. i]
            if row then
                row:HookScript("OnEnter", function(self) csHover = self end)
                row:HookScript("OnLeave", function(self) if csHover == self then csHover = nil end end)
            end
        end
        -- the rotate arrows keep their stock buttons (they turn the character while held down); only their look changes
        for _, spec in ipairs({ { CharacterSelectRotateLeft, true }, { CharacterSelectRotateRight, false } }) do
            local ob, left = spec[1], spec[2]
            if ob then
                for _, m in ipairs({ "GetNormalTexture", "GetPushedTexture", "GetHighlightTexture", "GetDisabledTexture" }) do
                    local t = ob[m](ob)
                    if t then t:SetAlpha(0) end
                end
                ob:ClearAllPoints()                                              -- below Enter World, a gap between the two
                ob:SetPoint("CENTER", GlueParent, "BOTTOM", left and -19 or 19, 18)
                local v = CreateFrame("Frame", nil, ob)
                v:SetFrameStrata("HIGH")
                v:EnableMouse(false)
                v:SetSize(30, 30)
                v:SetPoint("CENTER", ob, "CENTER", 0, 0)
                local base, hover, down = { 0.05, 0.06, 0.07, 0.78 }, { A * 0.30, B * 0.30, C * 0.30, 0.92 }, { A * 0.5, B * 0.5, C * 0.5, 0.95 }
                Flat(v, base, { A, B, C, 0.7 }, 12)
                local ic = v:CreateTexture(nil, "OVERLAY")
                ic:SetSize(20, 20); ic:SetPoint("CENTER", v, "CENTER", 0, 0)
                ic:SetTexture(FD .. "rotate.tga")
                ic:SetVertexColor(T.text[1], T.text[2], T.text[3])
                if left then ic:SetTexCoord(1, 0, 0, 1) end
                ob:HookScript("OnEnter", function() v:SetBackdropColor(hover[1], hover[2], hover[3], hover[4]) end)
                ob:HookScript("OnLeave", function() v:SetBackdropColor(base[1], base[2], base[3], base[4]) end)
                ob:HookScript("OnMouseDown", function() v:SetBackdropColor(down[1], down[2], down[3], down[4]) end)
                ob:HookScript("OnMouseUp", function() v:SetBackdropColor(hover[1], hover[2], hover[3], hover[4]) end)
            end
        end
        for _, b in ipairs(csButtons) do b:SetFrameStrata("HIGH"); if b.label then SetF(b.label, FONT_HEAD, 12) end end
        if ew and ew.label then SetF(ew.label, FONT_BLACK, 24) end
    end)
end

-- ---------------------------------------------------------------------------------------------------
-- 3. Keep things in step: window size changes, the login button's enabled state
-- ---------------------------------------------------------------------------------------------------
local sync = CreateFrame("Frame", nil, GlueParent)
local acc, lastLeft = 0, -1
sync:SetScript("OnUpdate", function(self, e)
    acc = acc + e
    if acc < 0.2 then return end
    acc = 0
    safe(function()
        local gl = GlueParent:GetLeft() or 0
        if math.abs(gl - lastLeft) > 0.5 then lastLeft = gl; LayoutColumns() end
        if panel and panel.login and AccountLoginLoginButton then
            local on = AccountLoginLoginButton:IsEnabled() and true or false
            if panel.login.on ~= on then
                panel.login.on = on
                panel.login:SetAlpha(on and 1 or 0.5)
            end
        end
        local screen = GetCurrentGlueScreenName and GetCurrentGlueScreenName() or "login"
        for _, c in ipairs(columns) do
            if c.dim then if screen == "login" then c.dim:Hide() else c.dim:Show() end end
        end
        for _, n in ipairs({ "OptionsButton", "OptionsButton2", "AccountLoginExitButton", "CharSelectChangeRealmButton", "CharSelectEnterWorldButton", "CharacterSelectBackButton", "CharacterSelectAddonsButton", "CharacterSelectDeleteButton", "CharSelectCreateCharacterButton", "DesenfoBoton", "DesenfoBoton2" }) do
            local b = _G[n]
            if b and csBuilt and b:GetAlpha() > 0.01 then b:SetAlpha(0) end
        end
        if not csBuilt and CharacterSelect and CharacterSelectUI and CharSelectEnterWorldButton then BuildCharSelect() end
        if selBox then
            local shown
            local sel = (CharacterSelect.selectedIndex or 0) - (WORKT_CHARACTER_OFFSET or 0)
            for i = 1, 8 do
                local row = _G["CharSelectCharacterButton" .. i]
                if row then
                    if row.selection then row.selection:SetAlpha(0) end
                    local on = _G[row:GetName() .. "OnMouse"]
                    if on then on:SetAlpha(0) end
                    local h = row:GetHighlightTexture()              -- the stock gold (selected, and under the mouse)
                    if h then h:SetAlpha(0) end
                    if row:IsShown() and sel == i and not shown then
                        selBox:ClearAllPoints(); selBox:SetPoint("CENTER", row, "CENTER", -22, 1); selBox:Show(); shown = true
                    end
                end
            end
            if not shown then selBox:Hide() end
            if hoverBox then
                if csHover and csHover:IsShown() and csHover:GetID() - (WORKT_CHARACTER_OFFSET or 0) ~= sel then
                    hoverBox:ClearAllPoints(); hoverBox:SetPoint("CENTER", csHover, "CENTER", -22, 1); hoverBox:Show()
                else
                    hoverBox:Hide()
                end
            end
        end
        for _, b in ipairs(csButtons) do                                   -- mirror enabled / disabled of the stock buttons
            local on = b.orig:IsEnabled() and true or false
            if b.on ~= on then b.on = on; b:SetAlpha(on and 1 or 0.5) end
            local hidden = b.hideAll and SelectBorroso == 0                -- the "hide all" view shows nothing but its eye
            if b.orig:IsVisible() and not hidden then b:Show() else b:Hide() end   -- visible when the stock button is
        end
        if csBuilt then
            for _, p in ipairs(WorkTCharacterPageButtons or {}) do if p:GetAlpha() > 0.01 then p:SetAlpha(0) end end
        end
        if csEyeAll then csEyeAll.icon:SetTexture(SelectBorroso == 0 and EYE_OFF or EYE) end
        if csEyeList then csEyeList.icon:SetTexture(SelectBorroso2 == 0 and EYE_OFF or EYE) end
        if csLogo then if SelectBorroso == 0 then csLogo:Hide() else csLogo:Show() end end
        -- the old news toggle is gone; keep stock layout code from showing the stock buttons
        if NewsButton and NewsButton:IsShown() then NewsButton:Hide() end
    end)
end)

__DEBUG__
