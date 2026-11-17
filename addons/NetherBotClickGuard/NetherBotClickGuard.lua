-- NetherBotClickGuard
-- Makes NPCBots (NPCBotPaperdoll / NetherBot) click through on right-click,
-- so you don't accidentally target them or pop open their menu while moving
-- the camera around in combat. Hold the configured modifier key (default
-- SHIFT) while right-clicking a bot to interact with it normally. Left-click
-- is left completely untouched.
--
-- NPCBots and alt bots join your party as creatures, but the client treats every
-- party member as a player. Right-clicking a bot in the world can open its
-- gossip, quest or merchant window, and right-clicking its frame shows the
-- standard unit menu ("Inspect / Trade / ..."). NPCBotPaperdoll hooks that
-- menu's "Inspect" entry to open its own bot panel.
--
-- Click-through: the stock client decides what a click hits in engine code before
-- Lua runs, so a plain addon cannot skip a unit. The From The Ashes client adds
-- WXL_SetClickThrough(button, modifier, unit, ...) for this: the addon hands it
-- the bots it knows (party/raid creatures, target, mouseover) and the engine's
-- click hit test then ignores them, so the click reaches whatever is behind
-- (ground, another NPC), unless the modifier key is held. The list is refreshed on
-- group changes and twice a second, since alt bots get a new GUID at each summon.
--
-- On a client without that function (stock or Windows) the older fallback below
-- still applies: we hook the generic Blizzard ToggleDropDownMenu function and,
-- the instant the resulting menu turns out to belong to one of our bots and was
-- opened by a world click, close it again unless the modifier was held; the
-- target is cleared and gossip/quest/merchant windows are closed the same way.
-- We deliberately do NOT hook UnitPopup_OnClick: NPCBotPaperdoll re-installs its
-- own wrapper around that function on a timer, so it will always end up as the
-- outermost hook and our wrapper would never run.

NetherBotClickGuardDB = NetherBotClickGuardDB or {}

local DEFAULTS = {
    modifier = "SHIFT",      -- "ALT", "SHIFT", or "CTRL"
    button = "RightButton",  -- "RightButton" or "LeftButton" - the click guarded/used to open the menu
    hideTooltip = true,      -- hide the unit tooltip for bots unless modifier held
    partyIsBot = true,       -- treat party/raid members whose GUID is a creature (NPCBots, alt bots) as bots
    trace = false,           -- print what the guard decides on each click (for troubleshooting)
    clickThrough = true,     -- engine click-through for bots (needs WXL_SetClickThrough in the client)
    whitelist = {},          -- [name] = true -> always treat as a bot
    blacklist = {},          -- [name] = true -> never treat as a bot
}

local function InitDB()
    for k, v in pairs(DEFAULTS) do
        if NetherBotClickGuardDB[k] == nil then
            if type(v) == "table" then
                NetherBotClickGuardDB[k] = {}
            else
                NetherBotClickGuardDB[k] = v
            end
        end
    end

    for _, list in ipairs({ "whitelist", "blacklist" }) do
        if type(NetherBotClickGuardDB[list]) ~= "table" then
            NetherBotClickGuardDB[list] = {}
        end
    end

    -- The saved-variables file can be edited by hand (game closed). Accept any
    -- capitalisation and fall back to the default instead of silently behaving
    -- as ALT when a value is not one of the valid choices.
    local modifier = string.upper(tostring(NetherBotClickGuardDB.modifier))
    if modifier ~= "ALT" and modifier ~= "SHIFT" and modifier ~= "CTRL" then
        modifier = DEFAULTS.modifier
    end
    NetherBotClickGuardDB.modifier = modifier

    local button = string.lower(tostring(NetherBotClickGuardDB.button))
    if button == "left" or button == "leftbutton" then
        NetherBotClickGuardDB.button = "LeftButton"
    else
        NetherBotClickGuardDB.button = DEFAULTS.button
    end
end

local function ModifierHeld()
    local mod = NetherBotClickGuardDB.modifier
    if mod == "SHIFT" then
        return IsShiftKeyDown()
    elseif mod == "CTRL" then
        return IsControlKeyDown()
    end
    return IsAltKeyDown()
end

local function Trace(msg)
    if NetherBotClickGuardDB and NetherBotClickGuardDB.trace then
        DEFAULT_CHAT_FRAME:AddMessage("|cff999999NBCG trace:|r " .. tostring(msg))
    end
end

-- True when `name` is someone else in your party or raid. NPCBots join your
-- group, and NPCBotPaperdoll's roster (below) is only filled when the server
-- sends it bot data, so group membership is the reliable signal. Names come
-- from the group roster, so this does not depend on the client resolving the
-- bot as a player unit.
local function IsGroupMemberName(name)
    if not name or name == "" then
        return false
    end
    if GetNumPartyMembers() == 0 and GetNumRaidMembers() == 0 then
        return false
    end
    local lower = string.lower(name)
    local me = UnitName("player")
    if me and string.lower(me) == lower then
        return false
    end
    for i = 1, 4 do
        local n = UnitName("party" .. i)
        if n and string.lower(n) == lower then
            return true
        end
    end
    for i = 1, 40 do
        local unit = "raid" .. i
        local n = UnitName(unit)
        if n and string.lower(n) == lower and not UnitIsUnit(unit, "player") then
            return true
        end
    end
    return false
end

-- UnitIsPlayer cannot be used for this: the client returns true for every party
-- member, creature or not. The GUID shows the real object type instead. Creature
-- GUIDs (NPCBots and alt bot projections) start with 0xF, player GUIDs with 0x0.
local function IsCreatureUnit(unit)
    local guid = unit and UnitGUID(unit)
    return type(guid) == "string" and guid:match("^0[xX][fF]") ~= nil
end

-- NPCBotPaperdoll keeps a bot roster in its own SavedVariables table, keyed per
-- character: NPCBotPaperdollDB[name].bots = { [botName] = { entry = ...,
-- isActive = ..., ... } }. It is only filled when the server sends it bot data,
-- so it can be empty; it is one of several ways to recognise a bot.
--
-- NPCBotPaperdoll works out its "current character" name once, when its file
-- first runs, and never recomputes it. On some clients and timings that is
-- before the real name is available, so its whole roster can end up filed under
-- the wrong key. Matching our own UnitName("player") against its keys is
-- therefore unreliable; we search every character bucket for an active bot with
-- the name we are checking instead.
--
-- `unit` is optional. The group rule needs it, because a name alone cannot tell
-- a bot from a real player; without a unit only the whitelist and
-- NPCBotPaperdoll's roster apply.
local function IsNetherBotName(name, unit)
    if not name or name == "" then
        return false
    end
    -- Hooks can fire before PLAYER_LOGIN has filled in the defaults.
    local db = NetherBotClickGuardDB
    if type(db) ~= "table" or type(db.blacklist) ~= "table" or type(db.whitelist) ~= "table" then
        return false
    end
    if db.blacklist[name] then
        return false
    end
    if db.whitelist[name] then
        return true
    end
    if NetherBotClickGuardDB.partyIsBot and unit and UnitExists(unit) and IsCreatureUnit(unit)
        and IsGroupMemberName(name) then
        return true
    end

    if type(NPCBotPaperdollDB) ~= "table" then
        return false
    end

    local lowerName = string.lower(name)
    for _, charDB in pairs(NPCBotPaperdollDB) do
        if type(charDB) == "table" and type(charDB.bots) == "table" then
            local bot = charDB.bots[name]
            if type(bot) == "table" and bot.entry and bot.isActive then
                return true
            end
            for botName, candidate in pairs(charDB.bots) do
                if type(botName) == "string" and string.lower(botName) == lowerName
                    and type(candidate) == "table" and candidate.entry and candidate.isActive then
                    return true
                end
            end
        end
    end

    return false
end

local function IsNetherBotUnit(unit)
    if not unit or not UnitExists(unit) then
        return false
    end
    return IsNetherBotName(UnitName(unit), unit)
end

-- Track whether the most recent target/menu change was caused by a
-- guarded-button click that landed on the 3D world (as opposed to clicking
-- a party frame, a raid frame, NPCBotPaperdoll's own roster UI, etc. -
-- those are deliberate UI actions and must never be guarded).
local recentWorldClick = false
local worldClickTimer = 0
local WORLD_CLICK_WINDOW = 0.3

-- Engine click-through. Units are passed as tokens, which the engine turns into
-- GUIDs on the spot, so the list always matches who is in the world right now.
local clickThroughUnits = {}
local clickThroughGUIDs = {}
local clickThroughTimer = 0
local CLICKTHROUGH_INTERVAL = 0.5

local function RefreshClickThrough()
    -- Before PLAYER_LOGIN has run InitDB the settings are not filled in yet.
    if type(WXL_SetClickThrough) ~= "function" or type(NetherBotClickGuardDB) ~= "table"
        or type(NetherBotClickGuardDB.modifier) ~= "string" or type(NetherBotClickGuardDB.button) ~= "string" then
        return
    end
    wipe(clickThroughUnits)
    wipe(clickThroughGUIDs)

    if NetherBotClickGuardDB.clickThrough then
        local function Consider(unit)
            if UnitExists(unit) and not UnitIsUnit(unit, "player") and IsNetherBotUnit(unit) then
                clickThroughUnits[#clickThroughUnits + 1] = unit
                local guid = UnitGUID(unit)
                if guid then
                    clickThroughGUIDs[guid] = true
                end
            end
        end
        for i = 1, 4 do
            Consider("party" .. i)
        end
        if GetNumRaidMembers() > 0 then
            for i = 1, 40 do
                Consider("raid" .. i)
            end
        end
        Consider("target")
        Consider("mouseover")
    end

    local button = NetherBotClickGuardDB.button == "LeftButton" and "left" or "right"
    local ok, err = pcall(WXL_SetClickThrough, button, string.lower(NetherBotClickGuardDB.modifier), unpack(clickThroughUnits))
    if not ok then
        Trace("WXL_SetClickThrough failed: " .. tostring(err))
    end
end

local guard = CreateFrame("Frame", "NetherBotClickGuardFrame", UIParent)
guard:SetScript("OnUpdate", function(self, elapsed)
    clickThroughTimer = clickThroughTimer + elapsed
    if clickThroughTimer > CLICKTHROUGH_INTERVAL then
        clickThroughTimer = 0
        RefreshClickThrough()
    end
    if recentWorldClick then
        worldClickTimer = worldClickTimer + elapsed
        if worldClickTimer > WORLD_CLICK_WINDOW then
            recentWorldClick = false
            worldClickTimer = 0
        end
    end
end)

local function OnWorldFrameClick(_, button)
    if button == NetherBotClickGuardDB.button then
        recentWorldClick = true
        worldClickTimer = 0
        Trace("world " .. tostring(button) .. " click seen")
    end
end
WorldFrame:HookScript("OnMouseDown", OnWorldFrameClick)
WorldFrame:HookScript("OnMouseUp", OnWorldFrameClick)

-- Tooltip suppression
hooksecurefunc(GameTooltip, "SetUnit", function(self)
    if not NetherBotClickGuardDB.hideTooltip then
        return
    end
    local _, unit = self:GetUnit()
    if unit and IsNetherBotUnit(unit) and not ModifierHeld() then
        self:Hide()
    end
end)

-- Pull unit/name context out of a dropdown the same way NPCBotPaperdoll
-- does for UnitPopup_OnClick, since the unit/name fields end up in
-- different places depending on how the menu was invoked.
local function GetDropdownContext(dropDownFrame)
    local unit, name

    if dropDownFrame then
        unit = dropDownFrame.unit
        name = dropDownFrame.name
    end

    if (not unit or unit == "") and UIDROPDOWNMENU_INIT_MENU then
        unit = unit or UIDROPDOWNMENU_INIT_MENU.unit
        name = name or UIDROPDOWNMENU_INIT_MENU.name
    end

    if (not unit or unit == "") and DropDownList1 and DropDownList1.dropdown then
        unit = unit or DropDownList1.dropdown.unit
        name = name or DropDownList1.dropdown.name
    end

    if unit and unit ~= "" and UnitExists(unit) then
        local unitName = UnitName(unit)
        if unitName and unitName ~= "" then
            name = unitName
        end
    end

    return unit, name
end

-- The actual fix: close the unit context menu (Inspect/Trade/.../the menu
-- NPCBotPaperdoll hijacks) the instant it opens for one of our bots from a
-- world click, unless the modifier is held.
hooksecurefunc("ToggleDropDownMenu", function(level, value, dropDownFrame, anchorName, xOffset, yOffset, menuList, button, ...)
    if not recentWorldClick or ModifierHeld() then
        return
    end

    local unit, name = GetDropdownContext(dropDownFrame)
    local isBot = IsNetherBotName(name, unit) or IsNetherBotUnit(unit)
    Trace("menu opened for " .. tostring(name) .. " (unit " .. tostring(unit) .. "): bot=" .. tostring(isBot))
    if isBot then
        CloseDropDownMenus()
    end
end)

-- Target revert + defensive auto-close for any gossip/quest/merchant window
-- a bot might still trigger directly (kept as a fallback layer).
guard:RegisterEvent("PLAYER_TARGET_CHANGED")
guard:RegisterEvent("GOSSIP_SHOW")
guard:RegisterEvent("QUEST_GREETING")
guard:RegisterEvent("MERCHANT_SHOW")
guard:RegisterEvent("PARTY_MEMBERS_CHANGED")
guard:RegisterEvent("RAID_ROSTER_UPDATE")
guard:RegisterEvent("PLAYER_ENTERING_WORLD")
guard:RegisterEvent("UPDATE_MOUSEOVER_UNIT")

-- The unit you are talking to ("npc") is checked as well as your target, since
-- an interaction does not always leave the NPC targeted.
local function InteractionIsBot()
    return IsNetherBotUnit("npc") or IsNetherBotUnit("target")
end

guard:SetScript("OnEvent", function(self, event)
    if event == "PARTY_MEMBERS_CHANGED" or event == "RAID_ROSTER_UPDATE" or event == "PLAYER_ENTERING_WORLD" then
        RefreshClickThrough()
        return
    elseif event == "UPDATE_MOUSEOVER_UNIT" then
        -- A bot that is not in the group (marked with /nbcg mark) is only found by hovering it.
        local guid = UnitExists("mouseover") and UnitGUID("mouseover")
        if guid and not clickThroughGUIDs[guid] and IsNetherBotUnit("mouseover") then
            RefreshClickThrough()
        end
        return
    end

    if ModifierHeld() then
        Trace(event .. ": modifier held, leaving it alone")
        return
    end

    if event == "PLAYER_TARGET_CHANGED" then
        local isBot = UnitExists("target") and IsNetherBotUnit("target")
        Trace("target changed to " .. tostring(UnitName("target")) .. ": bot=" .. tostring(isBot) .. ", world click=" .. tostring(recentWorldClick))
        if recentWorldClick and isBot then
            ClearTarget()
        end
    elseif event == "GOSSIP_SHOW" or event == "QUEST_GREETING" or event == "MERCHANT_SHOW" then
        local isBot = InteractionIsBot()
        Trace(event .. " with npc=" .. tostring(UnitName("npc")) .. " target=" .. tostring(UnitName("target")) .. ": bot=" .. tostring(isBot))
        if isBot then
            if event == "GOSSIP_SHOW" then
                CloseGossip()
            elseif event == "QUEST_GREETING" then
                CloseQuest()
            else
                CloseMerchant()
            end
        end
    end
end)

-- Slash command configuration
local function Print(msg)
    DEFAULT_CHAT_FRAME:AddMessage("|cff33ccffNetherBotClickGuard:|r " .. msg)
end

local function PrintUsage()
    Print("/nbcg modifier <alt|shift|ctrl> - set the key that allows clicking bots")
    Print("/nbcg button <left|right> - set which mouse button opens a bot's menu")
    Print("/nbcg tooltip <on|off> - show/hide tooltip on bots when modifier isn't held")
    Print("/nbcg mark - always treat your current mouseover unit as a bot")
    Print("/nbcg scan - mark everyone currently in your party/raid as a bot at once")
    Print("/nbcg unmark - remove your current mouseover unit from the bot list")
    Print("/nbcg ignore - never treat your current mouseover unit as a bot")
    Print("/nbcg unignore - remove your current mouseover unit from the ignore list")
    Print("/nbcg clickthrough <on|off> - bots are ignored by world clicks unless the modifier is held (needs the FTA client)")
    Print("/nbcg party <on|off> - treat creature party/raid members (NPCBots, alt bots) as bots (default on)")
    Print("/nbcg trace <on|off> - print what the guard decides on each click (troubleshooting)")
    Print("/nbcg debug - show whether your current mouseover unit is detected as a bot")
    Print("/nbcg status - show current settings")
end

SLASH_NETHERBOTCLICKGUARD1 = "/nbcg"
SlashCmdList["NETHERBOTCLICKGUARD"] = function(msg)
    msg = strtrim(msg or ""):lower()
    local cmd, arg = msg:match("^(%S*)%s*(.*)$")

    if cmd == "" or cmd == "help" then
        PrintUsage()
    elseif cmd == "modifier" then
        arg = arg:upper()
        if arg == "ALT" or arg == "SHIFT" or arg == "CTRL" then
            NetherBotClickGuardDB.modifier = arg
            Print("Modifier set to " .. arg .. ".")
            RefreshClickThrough()
        else
            Print("Usage: /nbcg modifier <alt|shift|ctrl>")
        end
    elseif cmd == "button" then
        if arg == "left" then
            NetherBotClickGuardDB.button = "LeftButton"
            Print("Left-click now opens a bot's menu (with modifier held); right-click is untouched.")
            RefreshClickThrough()
        elseif arg == "right" then
            NetherBotClickGuardDB.button = "RightButton"
            Print("Right-click now opens a bot's menu (with modifier held); left-click is untouched.")
            RefreshClickThrough()
        else
            Print("Usage: /nbcg button <left|right>")
        end
    elseif cmd == "tooltip" then
        if arg == "on" then
            NetherBotClickGuardDB.hideTooltip = false
            Print("Tooltip will always show.")
        elseif arg == "off" then
            NetherBotClickGuardDB.hideTooltip = true
            Print("Tooltip hidden on bots unless modifier is held.")
        else
            Print("Usage: /nbcg tooltip <on|off>")
        end
    elseif cmd == "clickthrough" then
        if arg == "on" or arg == "off" then
            NetherBotClickGuardDB.clickThrough = (arg == "on")
            RefreshClickThrough()
            if type(WXL_SetClickThrough) ~= "function" then
                Print("Saved, but this client has no WXL_SetClickThrough, so it has no effect here; bot menus are closed instead.")
            elseif arg == "on" then
                Print("Clicks pass straight through bots; hold " .. NetherBotClickGuardDB.modifier .. " to click one.")
            else
                Print("Clicks reach bots again; the old behaviour (close the menu it opens) applies.")
            end
        else
            Print("Usage: /nbcg clickthrough <on|off>")
        end
    elseif cmd == "party" then
        if arg == "on" then
            NetherBotClickGuardDB.partyIsBot = true
            Print("Party/raid members that are creatures (NPCBots, alt bots) are treated as bots; real players are left alone.")
        elseif arg == "off" then
            NetherBotClickGuardDB.partyIsBot = false
            Print("Only marked bots and NPCBotPaperdoll's roster are treated as bots.")
        else
            Print("Usage: /nbcg party <on|off>")
        end
    elseif cmd == "trace" then
        if arg == "on" then
            NetherBotClickGuardDB.trace = true
            Print("Trace on. Right-click a bot and watch for grey NBCG trace lines.")
        elseif arg == "off" then
            NetherBotClickGuardDB.trace = false
            Print("Trace off.")
        else
            Print("Usage: /nbcg trace <on|off>")
        end
    elseif cmd == "mark" or cmd == "unmark" or cmd == "ignore" or cmd == "unignore" then
        if not UnitExists("mouseover") then
            Print("Mouse over a unit first.")
            return
        end
        local name = UnitName("mouseover")
        if cmd == "mark" then
            NetherBotClickGuardDB.whitelist[name] = true
            NetherBotClickGuardDB.blacklist[name] = nil
            Print(name .. " will always be treated as a bot.")
        elseif cmd == "unmark" then
            NetherBotClickGuardDB.whitelist[name] = nil
            Print(name .. " removed from the bot list.")
        elseif cmd == "ignore" then
            NetherBotClickGuardDB.blacklist[name] = true
            NetherBotClickGuardDB.whitelist[name] = nil
            Print(name .. " will never be treated as a bot.")
        elseif cmd == "unignore" then
            NetherBotClickGuardDB.blacklist[name] = nil
            Print(name .. " removed from the ignore list.")
        end
    elseif cmd == "scan" then
        local marked = {}
        for i = 1, 4 do
            local unit = "party" .. i
            if UnitExists(unit) then
                local uname = UnitName(unit)
                if uname and not NetherBotClickGuardDB.blacklist[uname] then
                    NetherBotClickGuardDB.whitelist[uname] = true
                    table.insert(marked, uname)
                end
            end
        end
        for i = 1, 40 do
            local unit = "raid" .. i
            if UnitExists(unit) and not UnitIsUnit(unit, "player") then
                local uname = UnitName(unit)
                if uname and not NetherBotClickGuardDB.blacklist[uname] then
                    NetherBotClickGuardDB.whitelist[uname] = true
                    table.insert(marked, uname)
                end
            end
        end
        if next(marked) then
            Print("Marked as bots: " .. table.concat(marked, ", "))
            Print("If any of those are real players, use /nbcg ignore while hovering them.")
        else
            Print("No party/raid members found to mark. Group up with your bots first.")
        end
    elseif cmd == "debug" then
        if type(IsAddOnLoaded) == "function" then
            Print("NPCBotPaperdoll loaded: " .. tostring(IsAddOnLoaded("NPCBotPaperdoll")))
            Print("NetherBot loaded: " .. tostring(IsAddOnLoaded("NetherBot")))
        end
        local globalType = type(_G.NPCBotPaperdollDB)
        Print("NPCBotPaperdollDB global type: " .. globalType)
        local allBotNames = {}
        if globalType == "table" then
            local keys = {}
            for charKey, charDB in pairs(NPCBotPaperdollDB) do
                table.insert(keys, tostring(charKey))
                if type(charDB) == "table" and type(charDB.bots) == "table" then
                    for botName in pairs(charDB.bots) do
                        table.insert(allBotNames, tostring(botName) .. " (" .. tostring(charKey) .. ")")
                    end
                end
            end
            Print("Character keys in it: " .. (next(keys) and table.concat(keys, ", ") or "(none)"))
            Print("All bots across all keys: " .. (next(allBotNames) and table.concat(allBotNames, ", ") or "(none)"))
            Print("UnitName(\"player\") = " .. tostring(UnitName("player")))
        end
        if not UnitExists("mouseover") then
            Print("Mouse over a unit for per-unit info.")
            return
        end
        local name = UnitName("mouseover")
        local why = "not recognised"
        if NetherBotClickGuardDB.blacklist[name] then
            why = "ignored (/nbcg ignore)"
        elseif NetherBotClickGuardDB.whitelist[name] then
            why = "marked (/nbcg mark or scan)"
        elseif NetherBotClickGuardDB.partyIsBot and IsCreatureUnit("mouseover") and IsGroupMemberName(name) then
            why = "in your party/raid and a creature (NPCBot)"
        elseif IsNetherBotName(name, "mouseover") then
            why = "NPCBotPaperdoll roster"
        elseif IsGroupMemberName(name) then
            why = "in your party but its GUID is a player's, so not a bot"
        end
        Print(name .. ": isBot=" .. tostring(IsNetherBotName(name, "mouseover")) .. " (" .. why .. "), GUID=" .. tostring(UnitGUID("mouseover")))
        Print("npc=" .. tostring(UnitName("npc")) .. ", target=" .. tostring(UnitName("target")) .. ", party members=" .. tostring(GetNumPartyMembers()))
    elseif cmd == "status" then
        Print("Modifier: " .. NetherBotClickGuardDB.modifier)
        Print("Menu button: " .. NetherBotClickGuardDB.button)
        Print("Hide tooltip: " .. tostring(NetherBotClickGuardDB.hideTooltip))
        Print("Party members count as bots: " .. tostring(NetherBotClickGuardDB.partyIsBot))
        Print("Trace: " .. tostring(NetherBotClickGuardDB.trace))
        if type(WXL_SetClickThrough) ~= "function" then
            Print("Click-through: not available (this client has no WXL_SetClickThrough); menus are closed instead")
        else
            Print("Click-through: " .. (NetherBotClickGuardDB.clickThrough and "on" or "off") .. ", bots on the list now: " .. #clickThroughUnits)
        end
    else
        PrintUsage()
    end
end

local initFrame = CreateFrame("Frame")
initFrame:RegisterEvent("PLAYER_LOGIN")
initFrame:SetScript("OnEvent", function()
    InitDB()
    RefreshClickThrough()
    local buttonLabel = NetherBotClickGuardDB.button == "LeftButton" and "left-click" or "right-click"
    Print("Loaded. " .. buttonLabel .. " on a bot passes through; hold " .. NetherBotClickGuardDB.modifier .. " + " .. buttonLabel .. " to interact with it. Type /nbcg for options.")
end)
