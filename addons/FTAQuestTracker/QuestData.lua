local _, tracker = ...

local function present(value)
    return value ~= nil and value ~= ""
end

-- Wrath exposes the full sorted log through GetQuestLogTitle, even when
-- GetNumQuestLogEntries counts only rows shown under expanded headers.
function tracker.LogEntries()
    local visible, count = GetNumQuestLogEntries()
    local group, headerIndex, entries = QUEST_LOG, nil, {}
    for index = 1, math.max(visible or 0, 2 * (count or 0)) do
        local title, level, tag, partySize, header, collapsed, state, daily, id = GetQuestLogTitle(index)
        if header then
            group, headerIndex = title or QUEST_LOG, collapsed and index or nil
        elseif id and id > 0 then
            entries[#entries + 1] = {
                id = id, index = index, title = title, level = level, tag = tag, partySize = partySize,
                group = group, collapsedHeader = headerIndex, complete = state == 1, failed = state == -1,
                watched = IsQuestWatched(index) and true or false,
            }
        end
    end
    return entries
end

function tracker.FindQuest(id)
    for _, entry in ipairs(tracker.LogEntries()) do
        if entry.id == id then return entry end
    end
end

local function appendObjective(rows, text, done, current, total)
    rows[#rows + 1] = {text = text, done = done, current = current, total = total}
end

function tracker.ReadObjectives(quest)
    local rows, pending = {}, false
    local fromNative = C_FTA and C_FTA.GetQuestObjectives
    local structured = fromNative and fromNative(quest.id)
    if structured then
        for _, objective in ipairs(structured) do
            local text = objective.text
            if not present(text) then
                text = RETRIEVING_DATA or "Retrieving data"
                pending = true
            end
            appendObjective(rows, text, objective.finished, objective.numFulfilled, objective.numRequired)
        end
    else
        for i = 1, GetNumQuestLeaderBoards(quest.index) or 0 do
            local text, _, done = GetQuestLogLeaderBoard(i, quest.index)
            if not present(text) then
                text = RETRIEVING_DATA or "Retrieving data"
                pending = true
            end
            appendObjective(rows, text, done == 1 or done == true)
        end
    end

    if #rows == 0 and not quest.complete and not quest.failed then
        local text = GetQuestLogCompletionText(quest.index)
        if present(text) then
            appendObjective(rows, text, false)
        elseif fromNative and not structured then
            appendObjective(rows, RETRIEVING_DATA or "Retrieving data", false)
            pending = true
        end
    end

    local copper = GetQuestLogRequiredMoney(quest.index)
    if copper and copper > 0 then
        local money = GetCoinTextureString and GetCoinTextureString(copper, 12) or GetCoinText(copper)
        appendObjective(rows, money, GetMoney() >= copper)
    end
    return rows, pending
end
