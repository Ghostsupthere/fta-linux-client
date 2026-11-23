local name, tracker = ...
local width, height, step = 318, 460, 42
local minWidth, minHeight, maxWidth, maxHeight = 280, 180, 620, 760
local pendingRefresh, cacheRetries, settings
local defaultOpacity = 96
local slate = {0.13, 0.17, 0.21}
local gold = {1, 0.69, 0.36}
local white = {0.91, 0.94, 0.96}
local muted = {0.64, 0.71, 0.76}
local green = {0.55, 0.84, 0.59}
local red = {1, 0.48, 0.45}

local function text(parent, font, color)
    local label = parent:CreateFontString(nil, "OVERLAY", font)
    label:SetTextColor(unpack(color))
    label:SetJustifyH("LEFT")
    return label
end

local function fill(parent, layer, r, g, b, a)
    local texture = parent:CreateTexture(nil, layer)
    texture:SetAllPoints()
    texture:SetTexture(r, g, b, a)
    return texture
end

local frame = CreateFrame("Frame", "FTAQuestTrackerFrame", UIParent)
frame:SetSize(width, height)
frame:SetPoint("TOPRIGHT", UIParent, "TOPRIGHT", -286, -214)
frame:SetClampedToScreen(true)
frame:SetMovable(true)
frame:SetResizable(true)
frame:SetMinResize(minWidth, minHeight)
frame:SetMaxResize(maxWidth, maxHeight)
frame:EnableMouse(true)
frame:SetFrameStrata("MEDIUM")
frame:SetBackdrop({
    bgFile = "Interface\\Buttons\\WHITE8X8",
    edgeFile = "Interface\\Buttons\\WHITE8X8",
    edgeSize = 1,
    insets = {left = 1, right = 1, top = 1, bottom = 1},
})
frame:SetBackdropColor(0.054, 0.07, 0.088, 0.96)
frame:SetBackdropBorderColor(0.26, 0.31, 0.35, 1)
frame:Hide()

local function savePosition()
    local x, y = frame:GetCenter()
    local centerX, centerY = UIParent:GetCenter()
    settings.x, settings.y = x - centerX, y - centerY
end

local header = CreateFrame("Button", "FTAQuestTrackerHeader", frame)
header:SetPoint("TOPLEFT", 1, -1)
header:SetPoint("TOPRIGHT", -1, -1)
header:SetHeight(34)
header:EnableMouse(true)
local moving
header:SetScript("OnMouseDown", function(_, button)
    if button == "LeftButton" and settings and not settings.locked then
        moving = true
        frame:StartMoving()
    end
end)
header:SetScript("OnMouseUp", function(_, button)
    if button == "LeftButton" and moving then
        frame:StopMovingOrSizing()
        savePosition()
        moving = nil
    end
end)
local headerBackground = fill(header, "BACKGROUND", 0.12, 0.16, 0.20, 1)
local accent = header:CreateTexture(nil, "ARTWORK")
accent:SetPoint("TOPLEFT")
accent:SetPoint("BOTTOMLEFT")
accent:SetWidth(3)
accent:SetTexture(unpack(gold))
local title = text(header, "GameFontNormalLarge", white)
title:SetPoint("LEFT", 12, 0)
title:SetText("Quest Tracker")

local function button(parent, label, w, tooltip)
    local control = CreateFrame("Button", nil, parent)
    control:SetSize(w, 20)
    local caption = text(control, "GameFontHighlightSmall", muted)
    caption:SetPoint("CENTER")
    control:SetFontString(caption)
    control:SetText(label)
    control:SetHighlightTexture("Interface\\Buttons\\UI-Listbox-Highlight2", "ADD")
    if tooltip then
        control:SetScript("OnEnter", function(self)
            GameTooltip:SetOwner(self, "ANCHOR_BOTTOMLEFT")
            GameTooltip:SetText(tooltip)
        end)
        control:SetScript("OnLeave", function(self)
            if GameTooltip:IsOwned(self) then GameTooltip:Hide() end
        end)
    end
    return control
end

local close = button(header, "X", 20, "Hide tracker")
close:SetPoint("RIGHT", -7, 0)
local fold = button(header, "-", 20, "Collapse or expand tracker")
fold:SetPoint("RIGHT", close, "LEFT", -4, 0)
local filter = button(header, "Watched", 64, "Switch between watched quests and all active quests")
filter:SetPoint("RIGHT", fold, "LEFT", -6, 0)

local style = button(header, "Style", 40, "Tracker appearance and position")
style:SetPoint("RIGHT", filter, "LEFT", -4, 0)
local appearance = CreateFrame("Frame", "FTAQuestTrackerAppearance", frame)
appearance:SetPoint("TOPRIGHT", header, "BOTTOMRIGHT", -4, -4)
appearance:SetSize(226, 90)
appearance:SetFrameStrata("DIALOG")
appearance:EnableMouse(true)
appearance:SetBackdrop({
    bgFile = "Interface\\Buttons\\WHITE8X8",
    edgeFile = "Interface\\Buttons\\WHITE8X8",
    edgeSize = 1,
})
appearance:SetBackdropColor(0.075, 0.102, 0.129, 0.98)
appearance:SetBackdropBorderColor(0.26, 0.31, 0.35, 1)
appearance:Hide()
style:SetScript("OnClick", function()
    if appearance:IsShown() then appearance:Hide() else appearance:Show() end
end)

local opacity = CreateFrame("Slider", "FTAQuestTrackerOpacity", appearance, "OptionsSliderTemplate")
opacity:SetPoint("TOP", 0, -23)
opacity:SetWidth(190)
opacity:SetMinMaxValues(0, 100)
opacity:SetValueStep(1)
FTAQuestTrackerOpacityLow:SetText("0%")
FTAQuestTrackerOpacityHigh:SetText("100%")

local lock = CreateFrame("CheckButton", "FTAQuestTrackerLock", appearance, "UICheckButtonTemplate")
lock:SetPoint("BOTTOMLEFT", 12, 6)
lock:SetSize(24, 24)
local lockLabel = text(appearance, "GameFontHighlightSmall", white)
lockLabel:SetPoint("LEFT", lock, "RIGHT", 3, 0)
lockLabel:SetText("Lock position")
lock:SetScript("OnClick", function(self)
    settings.locked = self:GetChecked() and true or nil
end)
local stockFrameWasShown
local function hideStockTracker()
    if WatchFrame and WatchFrame:IsShown() then
        stockFrameWasShown = true
        WatchFrame:Hide()
    end
end

local resize = CreateFrame("Button", "FTAQuestTrackerResize", frame)
resize:SetSize(18, 18)
resize:SetPoint("BOTTOMRIGHT", -4, 4)
resize:SetNormalTexture("Interface\\ChatFrame\\UI-ChatIM-SizeGrabber-Up")
resize:SetHighlightTexture("Interface\\ChatFrame\\UI-ChatIM-SizeGrabber-Highlight")
local sizing
resize:SetScript("OnMouseDown", function(_, button)
    if button == "LeftButton" then
        sizing = true
        frame:StartSizing("BOTTOMRIGHT")
    end
end)
local function finishResize()
    frame:StopMovingOrSizing()
    settings.width, settings.height = frame:GetWidth(), frame:GetHeight()
    savePosition()
    sizing = nil
end
resize:SetScript("OnMouseUp", function(_, button)
    if button == "LeftButton" and sizing then finishResize() end
end)

frame:SetScript("OnShow", hideStockTracker)
frame:SetScript("OnHide", function()
    appearance:Hide()
    if moving then
        frame:StopMovingOrSizing()
        savePosition()
        moving = nil
    elseif sizing then
        finishResize()
    end
    if stockFrameWasShown and WatchFrame then WatchFrame:Show() end
    stockFrameWasShown = nil
end)


local scroll = CreateFrame("ScrollFrame", "FTAQuestTrackerScroll", frame, "UIPanelScrollFrameTemplate")
scroll:SetPoint("TOPLEFT", header, "BOTTOMLEFT", 9, -8)
scroll:SetPoint("BOTTOMRIGHT", frame, "BOTTOMRIGHT", -27, 20)
scroll:EnableMouseWheel(true)
local content = CreateFrame("Frame", nil, scroll)
content:SetSize(width - 37, 1)
scroll:SetScrollChild(content)
scroll:SetScript("OnMouseWheel", function(self, delta)
    local y = self:GetVerticalScroll() - delta * step
    self:SetVerticalScroll(math.max(0, math.min(self:GetVerticalScrollRange(), y)))
end)

local function queue()
    pendingRefresh = GetTime()
    cacheRetries = 0
end

local function toggleWatch(id)
    local quest = tracker.FindQuest(id)
    if not quest then return end
    if quest.watched then
        RemoveQuestWatch(quest.index)
    else
        AddQuestWatch(quest.index)
    end
    queue()
end

local function openQuest(id)
    local quest = tracker.FindQuest(id)
    if not quest then return end
    if quest.collapsedHeader then
        ExpandQuestHeader(quest.collapsedHeader)
        quest = tracker.FindQuest(id)
        if not quest then return end
    end
    local log = QuestLogExFrame or ClassicQuestLog or QuestLogFrame
    if not log or not log:IsShown() then ToggleQuestLog() end
    if QuestLog_SetSelection then QuestLog_SetSelection(quest.index) end
    SelectQuestLogEntry(quest.index)
    if QuestLog_Update then QuestLog_Update() end
end

local pools, used = {group = {}, quest = {}, objective = {}, notice = {}}, {}

local function applyOpacity(percent)
    local alpha = percent / 100
    frame:SetBackdropColor(0.054, 0.07, 0.088, alpha)
    frame:SetBackdropBorderColor(0.26, 0.31, 0.35, alpha)
    headerBackground:SetAlpha(alpha)
    for _, rows in ipairs({pools.group, pools.quest}) do
        for _, row in ipairs(rows) do row.background:SetAlpha(alpha) end
    end
    FTAQuestTrackerOpacityText:SetText(("Background opacity %d%%"):format(percent))
end

opacity:SetScript("OnValueChanged", function(_, value)
    if not settings then return end
    settings.opacity = math.floor(value + 0.5)
    applyOpacity(settings.opacity)
end)

local function acquire(kind)
    local cursor = (used[kind] or 0) + 1
    used[kind] = cursor
    local row = pools[kind][cursor]
    if row then return row end
    row = CreateFrame((kind == "group" or kind == "quest") and "Button" or "Frame", nil, content)
    pools[kind][cursor] = row
    row.label = text(row, "GameFontHighlightSmall", white)
    row.label:SetPoint("TOPLEFT", 9, -5)
    row.label:SetWidth(content:GetWidth() - 17)
    if kind == "group" or kind == "quest" then
        row:SetHighlightTexture("Interface\\Buttons\\UI-Listbox-Highlight2", "ADD")
        if kind == "group" then
            row.background = fill(row, "BACKGROUND", slate[1], slate[2], slate[3], 1)
            row.background:SetAlpha(settings.opacity / 100)
            row:SetScript("OnClick", function(self)
                settings.groups[self.group] = not settings.groups[self.group] or nil
                queue()
            end)
        else
            row.background = fill(row, "BACKGROUND", 0.09, 0.12, 0.15, 0.96)
            row.background:SetAlpha(settings.opacity / 100)
            row:RegisterForClicks("LeftButtonUp", "RightButtonUp")
            row:SetScript("OnClick", function(self, mouse)
                if mouse == "RightButton" then toggleWatch(self.questID) else openQuest(self.questID) end
            end)
            row:SetScript("OnEnter", function(self)
                local quest = tracker.FindQuest(self.questID)
                if not quest then return end
                GameTooltip:SetOwner(self, "ANCHOR_LEFT")
                GameTooltip:SetText(quest.title or QUEST_LOG, 1, 0.81, 0.54, 1, true)
                if quest.tag then GameTooltip:AddLine(quest.tag) end
                GameTooltip:AddLine("Left-click to open in the quest log", 0.85, 0.89, 0.92)
                GameTooltip:AddLine(quest.watched and "Right-click to stop tracking" or "Right-click to track", 0.70, 0.77, 0.81)
                GameTooltip:Show()
            end)
            row:SetScript("OnLeave", function(self)
                if GameTooltip:IsOwned(self) then GameTooltip:Hide() end
            end)

            local item = CreateFrame("Button", nil, row)
            item:SetSize(21, 21)
            item:SetPoint("RIGHT", -4, 0)
            item:SetHighlightTexture("Interface\\Buttons\\ButtonHilight-Square", "ADD")
            item.icon = item:CreateTexture(nil, "ARTWORK")
            item.icon:SetAllPoints()
            item.cooldown = CreateFrame("Cooldown", nil, item, "CooldownFrameTemplate")
            item.cooldown:SetAllPoints()
            item:SetScript("OnClick", function()
                local quest = tracker.FindQuest(row.questID)
                if quest and GetQuestLogSpecialItemInfo(quest.index) then UseQuestLogSpecialItem(quest.index) end
            end)
            item:SetScript("OnEnter", function(self)
                local quest = tracker.FindQuest(row.questID)
                local link = quest and GetQuestLogSpecialItemInfo(quest.index)
                if link then
                    GameTooltip:SetOwner(self, "ANCHOR_LEFT")
                    GameTooltip:SetHyperlink(link)
                    GameTooltip:Show()
                end
            end)
            item:SetScript("OnLeave", function(self)
                if GameTooltip:IsOwned(self) then GameTooltip:Hide() end
            end)
            row.item = item
        end
    elseif kind == "objective" then
        row.dot = row:CreateTexture(nil, "ARTWORK")
        row.dot:SetSize(4, 4)
        row.dot:SetPoint("TOPLEFT", 12, -9)
        row.label:ClearAllPoints()
        row.label:SetPoint("TOPLEFT", 23, -5)
        row.label:SetWidth(content:GetWidth() - 30)
        row.progress = CreateFrame("StatusBar", nil, row)
        row.progress:SetPoint("BOTTOMLEFT", 23, 3)
        row.progress:SetPoint("BOTTOMRIGHT", -12, 3)
        row.progress:SetHeight(3)
        row.progress:SetStatusBarTexture("Interface\\TargetingFrame\\UI-StatusBar")
        fill(row.progress, "BACKGROUND", 0.16, 0.21, 0.25, 1)
    end
    return row
end

local function place(row, offset, rowHeight, gap)
    row:ClearAllPoints()
    row:SetPoint("TOPLEFT", content, "TOPLEFT", 0, -offset)
    row:SetPoint("TOPRIGHT", content, "TOPRIGHT", 0, -offset)
    row:SetHeight(rowHeight)
    row:Show()
    return offset + rowHeight + (gap or 0)
end

local function addQuest(quest, offset)
    local row = acquire("quest")
    row.questID = quest.id
    local _, icon = GetQuestLogSpecialItemInfo(quest.index)
    if icon then
        row.item.icon:SetTexture(icon)
        local start, duration, enabled = GetQuestLogSpecialItemCooldown(quest.index)
        CooldownFrame_SetTimer(row.item.cooldown, start or 0, duration or 0, enabled or 0)
        row.item:Show()
    else
        row.item:Hide()
    end
    row.label:SetWidth(content:GetWidth() - (icon and 39 or 17))
    local status = quest.failed and "  Failed" or quest.complete and "  Ready to turn in" or ""
    local level = quest.level > 0 and ("[" .. quest.level .. "] ") or ""
    row.label:SetText(level .. (quest.title or RETRIEVING_DATA or "Retrieving data") .. status)
    row.label:SetTextColor(unpack(quest.failed and red or quest.complete and green or white))
    local labelHeight = row.label:GetStringHeight()
    offset = place(row, offset, math.max(24, labelHeight + 11), 1)

    local objectives, pending = tracker.ReadObjectives(quest)
    for _, objective in ipairs(objectives) do
        if not settings.hideCompleted or not objective.done or quest.failed then
            local entry = acquire("objective")
            local color = quest.failed and red or objective.done and green or muted
            entry.label:SetText(objective.text)
            entry.label:SetTextColor(unpack(color))
            entry.dot:SetTexture(color[1], color[2], color[3], 1)
            local progress = not quest.failed and not objective.done and objective.total and objective.total > 1
            if progress then
                entry.progress:SetMinMaxValues(0, objective.total)
                entry.progress:SetValue(math.max(0, math.min(objective.total, objective.current or 0)))
                entry.progress:SetStatusBarColor(gold[1], gold[2], gold[3])
                entry.progress:Show()
            else
                entry.progress:Hide()
            end
            offset = place(entry, offset, math.max(15, entry.label:GetStringHeight() + (progress and 13 or 7)))
        end
    end
    return offset + 5, pending
end

local function render()
    for kind, rows in pairs(pools) do
        used[kind] = 0
        for _, row in ipairs(rows) do row:Hide() end
    end

    local offset, group, pending, count = 0, nil, false, 0
    for _, quest in ipairs(tracker.LogEntries()) do
        if settings.all or quest.watched then
            if quest.group ~= group then
                if group then offset = offset + 2 end
                group = quest.group
                local heading = acquire("group")
                heading.group = group
                heading.label:SetText((settings.groups[group] and "+  " or "-  ") .. group)
                heading.label:SetTextColor(unpack(gold))
                offset = place(heading, offset, 22, 4)
            end
            count = count + 1
            if not settings.groups[group] then
                local needsData
                offset, needsData = addQuest(quest, offset)
                pending = pending or needsData
            end
        end
    end

    if count == 0 then
        local row = acquire("notice")
        row.label:SetText(settings.all and "No active quests." or "No watched quests. Select All to view your quest log.")
        row.label:SetTextColor(unpack(muted))
        offset = place(row, 0, 38)
    end
    content:SetHeight(math.max(1, offset))
    scroll:SetVerticalScroll(math.min(scroll:GetVerticalScroll(), scroll:GetVerticalScrollRange()))
    return pending
end

frame:SetScript("OnSizeChanged", function(self)
    local listWidth = self:GetWidth() - 37
    content:SetWidth(listWidth)
    for kind, rows in pairs(pools) do
        local inset = kind == "objective" and 30 or 17
        for _, row in ipairs(rows) do row.label:SetWidth(listWidth - inset) end
    end
    if settings then queue() end
end)

local function setExpanded()
    frame:SetHeight(settings.collapsed and 36 or settings.height or height)
    fold:SetText(settings.collapsed and "+" or "-")
    if settings.collapsed then
        scroll:Hide()
        resize:Hide()
    else
        scroll:Show()
        resize:Show()
    end
    queue()
end

local function setTrackerVisible(visible)
    settings.hidden = not visible or nil
    if visible then frame:Show() else frame:Hide() end
end

local function toggleTracker()
    setTrackerVisible(not frame:IsShown())
end

local function createMinimapButton()
    local launcher = CreateFrame("Button", "FTAQuestTrackerMinimapButton", Minimap)
    launcher:SetSize(31, 31)
    launcher:SetFrameLevel(Minimap:GetFrameLevel() + 8)
    launcher:SetHighlightTexture("Interface\\Minimap\\UI-Minimap-ZoomButton-Highlight")
    launcher:RegisterForClicks("LeftButtonUp", "RightButtonUp")
    launcher:RegisterForDrag("LeftButton")

    local icon = launcher:CreateTexture(nil, "ARTWORK")
    icon:SetTexture("Interface\\Icons\\INV_Misc_Note_01")
    icon:SetSize(20, 20)
    icon:SetPoint("TOPLEFT", 7, -5)
    icon:SetTexCoord(0.05, 0.95, 0.05, 0.95)
    local border = launcher:CreateTexture(nil, "OVERLAY")
    border:SetTexture("Interface\\Minimap\\MiniMap-TrackingBorder")
    border:SetSize(53, 53)
    border:SetPoint("TOPLEFT")

    local function position()
        if launcher:GetParent() ~= Minimap then return end
        local angle = tonumber(settings.minimapAngle) or math.pi
        launcher:SetPoint("CENTER", Minimap, "CENTER",
            math.cos(angle) * (Minimap:GetWidth() / 2 + 6),
            math.sin(angle) * (Minimap:GetHeight() / 2 + 6))
    end

    local function stopDrag()
        launcher:SetScript("OnUpdate", nil)
    end

    launcher:SetScript("OnClick", function(_, mouse)
        if mouse == "RightButton" then
            setTrackerVisible(true)
            appearance:Show()
        else
            toggleTracker()
        end
    end)
    launcher:SetScript("OnEnter", function(self)
        GameTooltip:SetOwner(self, "ANCHOR_LEFT")
        GameTooltip:SetText("FTA Quest Tracker")
        GameTooltip:AddLine("Left-click to show or hide the tracker.", 1, 1, 1)
        GameTooltip:AddLine("Right-click to open Style settings.", 1, 1, 1)
        if self:GetParent() == Minimap then
            GameTooltip:AddLine("Drag to move the button.", 1, 1, 1)
        end
        GameTooltip:Show()
    end)
    launcher:SetScript("OnLeave", function(self)
        if GameTooltip:IsOwned(self) then GameTooltip:Hide() end
    end)
    launcher:SetScript("OnDragStart", function(self)
        if self:GetParent() ~= Minimap then return end
        GameTooltip:Hide()
        self:SetScript("OnUpdate", function()
            if self:GetParent() ~= Minimap then
                stopDrag()
                return
            end
            local centerX, centerY = Minimap:GetCenter()
            if not centerX then return end
            local x, y = GetCursorPosition()
            local scale = Minimap:GetEffectiveScale()
            settings.minimapAngle = math.atan2(y / scale - centerY, x / scale - centerX)
            position()
        end)
    end)
    launcher:SetScript("OnDragStop", stopDrag)
    launcher:SetScript("OnHide", stopDrag)
    Minimap:HookScript("OnSizeChanged", position)
    position()
end

close:SetScript("OnClick", function() setTrackerVisible(false) end)
fold:SetScript("OnClick", function() settings.collapsed = not settings.collapsed or nil; setExpanded() end)
filter:SetScript("OnClick", function()
    settings.all = not settings.all or nil
    filter:SetText(settings.all and "All" or "Watched")
    scroll:SetVerticalScroll(0)
    queue()
end)

frame:SetScript("OnUpdate", function()
    -- Blizzard can show its WatchFrame again after quest and UI events.
    hideStockTracker()
    local now = GetTime()
    if settings.collapsed or not pendingRefresh or now < pendingRefresh then return end
    pendingRefresh = nil
    if render() and cacheRetries < 4 then
        cacheRetries = cacheRetries + 1
        pendingRefresh = now + 0.55 * cacheRetries
    end
end)

local events = CreateFrame("Frame")
events:RegisterEvent("ADDON_LOADED")
events:SetScript("OnEvent", function(self, event, addon)
    if event ~= "ADDON_LOADED" then
        queue()
        return
    end
    if addon ~= name then return end
    self:UnregisterEvent(event)
    if type(FTAQuestTrackerSettings) ~= "table" then FTAQuestTrackerSettings = {} end
    settings = FTAQuestTrackerSettings
    settings.opacity = math.max(0, math.min(100, math.floor((tonumber(settings.opacity) or defaultOpacity) + 0.5)))
    opacity:SetValue(settings.opacity)
    applyOpacity(settings.opacity)
    lock:SetChecked(settings.locked and 1 or nil)
    if type(settings.groups) ~= "table" then settings.groups = {} end
    if type(settings.x) == "number" and type(settings.y) == "number" then
        frame:ClearAllPoints()
        frame:SetPoint("CENTER", UIParent, "CENTER", settings.x, settings.y)
    end
    settings.width = math.max(minWidth, math.min(maxWidth, tonumber(settings.width) or width))
    settings.height = math.max(minHeight, math.min(maxHeight, tonumber(settings.height) or height))
    frame:SetWidth(settings.width)
    filter:SetText(settings.all and "All" or "Watched")
    setExpanded()
    setTrackerVisible(not settings.hidden)
    createMinimapButton()
    for _, signal in ipairs({"PLAYER_ENTERING_WORLD", "QUEST_LOG_UPDATE", "QUEST_WATCH_UPDATE",
        "QUEST_ITEM_UPDATE", "BAG_UPDATE", "BAG_UPDATE_COOLDOWN", "PLAYER_MONEY"}) do
        self:RegisterEvent(signal)
    end
    hooksecurefunc("AddQuestWatch", queue)
    hooksecurefunc("RemoveQuestWatch", queue)

    SLASH_FTAQUESTTRACKER1 = "/ftaqt"
    SlashCmdList.FTAQUESTTRACKER = function(input)
        local command = (input or ""):trim():lower()
        if command == "completed" then
            settings.hideCompleted = not settings.hideCompleted or nil
            print(settings.hideCompleted and "Quest tracker hides completed objectives." or "Quest tracker shows completed objectives.")
        elseif command == "reset" then
            settings.x, settings.y = nil, nil
            settings.width, settings.height = width, height
            frame:SetWidth(width)
            setExpanded()
            frame:ClearAllPoints()
            frame:SetPoint("TOPRIGHT", UIParent, "TOPRIGHT", -286, -214)
            setTrackerVisible(true)
        elseif command == "" then
            toggleTracker()
        else
            print("/ftaqt toggles the tracker. /ftaqt completed toggles completed objectives. /ftaqt reset restores its position and size.")
        end
        queue()
    end
end)
