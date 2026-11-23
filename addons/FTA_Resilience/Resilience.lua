-- Also bundled with AllStats so sharing that addon retains its corrected tooltips.
if FTA_Resilience then return end

local api = { ceiling = 66, curve = 11, confirmed = false }
FTA_Resilience = api
-- Stock, AllStats and Drakeish use the same setter with different stat frames.
-- Keep the displayed rows current when the server confirms or changes its curve.
local rows = setmetatable({}, { __mode = "k" })

local function finite(value)
    return type(value) == "number" and value == value and value ~= math.huge and value ~= -math.huge
end

function api.Reduction(points)
    if not finite(points) or points <= 0 then return 0 end
    return math.min(api.ceiling, math.max(0, api.ceiling * (1 - math.exp(-points / api.curve))))
end

function api.Values()
    local rating, points = 0, 0
    for _, index in ipairs({ CR_CRIT_TAKEN_MELEE or 15, CR_CRIT_TAKEN_RANGED or 16,
        CR_CRIT_TAKEN_SPELL or 17 }) do
        local raw = GetCombatRating(index) or 0
        local bonus = GetCombatRatingBonus(index) or 0
        if finite(raw) then rating = math.max(rating, raw) end
        if finite(bonus) then points = math.max(points, bonus) end
    end
    return rating, points, api.Reduction(points)
end

function api.Description()
    local _, _, reduction = api.Values()
    local status = api.confirmed and "Server settings" or "Default curve; awaiting server confirmation"
    return string.format("Reduces qualifying PvP damage by %.2f%%.\n" ..
        "The same percentage applies to normal hits, critical hits, spells and periodic damage.\n" ..
        "Applies against players, NPCBots and their controlled pets or summons.\n" ..
        "Does not reduce critical-hit chance or add separate critical-damage reduction.\n" ..
        "Does not mitigate ordinary PvE, environmental or self-inflicted damage.\n" ..
        "Diminishing returns approach %.2f%%; additional rating continues to help.\n%s.",
        reduction, api.ceiling, status)
end

function api.ShowTooltip(frame)
    local rating, points, reduction = api.Values()
    if not api.confirmed then return end
    GameTooltip:SetOwner(frame, "ANCHOR_RIGHT")
    GameTooltip:ClearLines()
    GameTooltip:AddLine("PvP Resilience", 1, 0.82, 0)
    GameTooltip:AddDoubleLine("Rating", string.format("%.0f", rating), 1, 1, 1, 1, 1, 1)
    GameTooltip:AddDoubleLine("PvP damage reduction", string.format("%.2f%%", reduction), 1, 1, 1, 0.4, 1, 0.4)
    GameTooltip:AddLine(api.Description(), 1, 1, 1, true)
    GameTooltip:AddLine(" ")
    GameTooltip:AddLine(string.format("Level-adjusted R: %.4f. Formula: %.2f * (1 - exp(-R / %.4f)).",
        points, api.ceiling, api.curve), 0.7, 0.7, 0.7, true)
    GameTooltip:AddLine("Resilience no longer supplies PvE critical-hit immunity. Use defense or appropriate talents.",
        1, 0.65, 0.3, true)
    GameTooltip:AddLine("Legacy mana-drain protection is unchanged.", 0.7, 0.7, 0.7, true)
    GameTooltip:Show()
end

function api.UpdateRow(frame)
    if not frame then return end
    rows[frame] = true
    if not api.confirmed then return end
    local rating = api.Values()
    PaperDollFrame_SetLabelAndText(frame, STAT_RESILIENCE, rating)
    frame.tooltip = "PvP Resilience: " .. string.format("%.0f", rating)
    frame.tooltip2 = api.Description()
    if frame.HookScript and not frame._ftaResilienceTooltip then
        frame._ftaResilienceTooltip = true
        frame:HookScript("OnEnter", api.ShowTooltip)
    end
end

function api.Refresh()
    if PaperDollFrame_UpdateStats then PaperDollFrame_UpdateStats() end
    for row in pairs(rows) do api.UpdateRow(row) end
    -- A server reply must also refresh an already-open modern stat tooltip.
    if GameTooltip and GameTooltip.IsShown and GameTooltip:IsShown() and GameTooltip.GetOwner then
        local owner = GameTooltip:GetOwner()
        if owner and rows[owner] then api.ShowTooltip(owner) end
    end
end

if hooksecurefunc and PaperDollFrame_SetResilience then
    hooksecurefunc("PaperDollFrame_SetResilience", api.UpdateRow)
end

local frame = CreateFrame("Frame")
frame:RegisterEvent("PLAYER_ENTERING_WORLD")
frame:RegisterEvent("CHAT_MSG_ADDON")
local elapsed = 0

local function request()
    local name = UnitName("player")
    if name and SendAddonMessage then SendAddonMessage("FRS1", "Q", "WHISPER", name) end
end

frame:SetScript("OnEvent", function(_, event, prefix, message, channel, sender)
    if event == "PLAYER_ENTERING_WORLD" then
        api.ceiling, api.curve, api.confirmed = 66, 11, false
        elapsed = 0
        api.Refresh()
        request()
    elseif prefix == "FRS1" and channel == "WHISPER" and sender == UnitName("player") then
        local ceiling, curve = string.match(message or "", "^C\t([^\t]+)\t([^\t]+)$")
        ceiling, curve = tonumber(ceiling), tonumber(curve)
        if not finite(ceiling) or ceiling < 0 or ceiling > 66 or not finite(curve) or curve <= 0 then return end
        api.ceiling, api.curve, api.confirmed = ceiling, curve, true
        api.Refresh()
    end
end)

-- Configuration only: no model queries, gear scans or shapeshift snapshot requests.
frame:SetScript("OnUpdate", function(_, delta)
    elapsed = elapsed + delta
    if elapsed >= (api.confirmed and 60 or 5) then
        elapsed = 0
        request()
    end
end)
