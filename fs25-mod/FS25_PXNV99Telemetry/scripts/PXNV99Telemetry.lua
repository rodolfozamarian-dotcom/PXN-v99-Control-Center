-- PXN V99 Telemetry
--
-- Envia, a até 100 Hz, uma linha JSON por quadro com os dados do veículo que o
-- jogador está dirigindo para o app PXN V99 Control Center, pelo named pipe
-- \\.\pipe\PXNV99Telemetry (o mesmo mecanismo usado pelo mod de telemetria do
-- SimHub). O mod só lê o estado do jogo; não altera nada.
--
-- Protocolo (v = 1), todos os campos opcionais:
--   t     tempo do jogo (ms)            in    jogador dentro de um veículo
--   pz    jogo pausado ou menu aberto   on    motor ligado
--   rpm   rotação atual                 rmin  / rmax  rotação mínima / máxima
--   ld    carga do motor 0..1           thr   acelerador -1..1
--   clt   pedal de embreagem 0..1       gr    marcha atual
--   spd   velocidade km/h (com sinal)   wn    rodas / wc rodas com contato
--   gRd / gHd / gFd / gSf   rodas em asfalto / terra firme / lavoura / terra fofa
--   dep   afundamento médio no solo     slp   patinagem média das rodas
--   csl   embreagem patinando 0..1      stl   motor afogando
--   trq   torque aplicado / máximo      sus   velocidade média da suspensão (m/s)
--   wat   rodas na água                 sfl / sfr  velocidade da suspensão dianteira esq./dir. (m/s)
--   msg   mensagem de diagnóstico

PXNV99Telemetry = {}

local PIPE = "\\\\.\\pipe\\PXNV99Telemetry"
local SEND_INTERVAL_MS = 10      -- 100 Hz no máximo
local IDLE_INTERVAL_MS = 250     -- fora do veículo: só um sinal de vida
local RETRY_INTERVAL_MS = 2000   -- app fechado: tentar abrir o pipe de novo
-- O io do jogo nem sempre informa que o pipe fechou (ex.: app reiniciado).
-- Reabrir de tempos em tempos garante a reconexão; o app mantém uma instância
-- extra do pipe à espera, então a troca não interrompe os dados.
local REOPEN_INTERVAL_MS = 10000

local state = {
    file = nil,
    sinceSend = 0,
    sinceRetry = RETRY_INTERVAL_MS,
    sinceOpen = 0,
    online = false,          -- já anunciou a conexão no log
    loggedWriteResult = false,
    dumpedVehicle = nil,
    -- Comprimento anterior da suspensão de cada roda, para medir a trepidação.
    prevSusp = setmetatable({}, { __mode = "k" }),
    lastFrameTime = nil,
}

local function log(fmt, ...)
    print(string.format("[PXNV99] " .. fmt, ...))
end

local function closePipe()
    if state.file ~= nil then
        pcall(function() state.file:close() end)
        state.file = nil
    end
end

local function openPipe()
    local ok, f = pcall(io.open, PIPE, "w")
    if ok and f ~= nil then
        state.file = f
        state.sinceOpen = 0
        if not state.online then
            state.online = true
            log("conectado ao PXN V99 Control Center")
        end
        return true
    end
    return false
end

local function send(line)
    if state.file == nil then
        return
    end
    -- Só um erro explícito conta como queda. No FS25, flush() não devolve nada
    -- mesmo quando dá certo; tratar isso como falha (mod 0.3.1) derrubava a
    -- conexão a cada quadro e a vibração chegava em pulsos.
    local okCall, r1, r2 = pcall(state.file.write, state.file, line .. "\n")
    pcall(state.file.flush, state.file)
    if not state.loggedWriteResult then
        state.loggedWriteResult = true
        log("diagnóstico: write devolveu (%s, %s)", tostring(r1), tostring(r2))
    end
    if not okCall or (r1 == nil and r2 ~= nil) then
        log("conexão com o app perdida (%s); tentando reconectar a cada %d s", tostring(okCall and r2 or r1), RETRY_INTERVAL_MS / 1000)
        closePipe()
        state.online = false
        state.sinceRetry = 0
    end
end

local function num(v)
    if type(v) ~= "number" or v ~= v or v == math.huge or v == -math.huge then
        return nil
    end
    return v
end

local function currentVehicle()
    if g_localPlayer ~= nil and g_localPlayer.getCurrentVehicle ~= nil then
        local ok, v = pcall(g_localPlayer.getCurrentVehicle, g_localPlayer)
        if ok and v ~= nil then
            return v
        end
    end
    local m = g_currentMission
    if m ~= nil then
        if m.controlledVehicle ~= nil then
            return m.controlledVehicle
        end
        if m.getControlledVehicle ~= nil then
            local ok, v = pcall(m.getControlledVehicle, m)
            if ok then
                return v
            end
        end
    end
    return nil
end

local function isPaused()
    local m = g_currentMission
    if m ~= nil and m.paused then
        return true
    end
    if g_gui ~= nil and g_gui.getIsGuiVisible ~= nil then
        local ok, visible = pcall(g_gui.getIsGuiVisible, g_gui)
        if ok and visible then
            return true
        end
    end
    return false
end

-- Motor ligado? No FS25 o campo `isMotorStarted` não existe mais; o estado
-- fica em `motorState` (enum MotorState) e na função getIsMotorStarted.
local function motorOn(vehicle, motorized)
    if vehicle.getIsMotorStarted ~= nil then
        local ok, started = pcall(vehicle.getIsMotorStarted, vehicle)
        if ok and type(started) == "boolean" then
            return started
        end
    end
    if MotorState ~= nil and MotorState.ON ~= nil and motorized.motorState ~= nil then
        return motorized.motorState == MotorState.ON
    end
    if type(motorized.isMotorStarted) == "boolean" then
        return motorized.isMotorStarted
    end
    return nil
end

-- Classe do solo de uma roda, pela mesma regra do jogo (WheelsUtil.getGroundType).
local function groundClass(p)
    if p == nil or WheelsUtil == nil or WheelsUtil.getGroundType == nil then
        return nil
    end
    local isField = FieldGroundType ~= nil and p.densityType ~= nil and p.densityType ~= FieldGroundType.NONE
    local notGround = WheelContactType ~= nil and p.contact ~= nil and p.contact ~= WheelContactType.GROUND
    local ok, gt = pcall(WheelsUtil.getGroundType, isField, notGround, tonumber(p.groundDepth) or 0)
    if not ok then
        return nil
    end
    if gt == WheelsUtil.GROUND_ROAD then
        return "gRd"
    elseif gt == WheelsUtil.GROUND_FIELD then
        return "gFd"
    elseif gt == WheelsUtil.GROUND_SOFT_TERRAIN then
        return "gSf"
    end
    return "gHd"
end

local function hasContact(p)
    if p == nil then
        return false
    end
    if p.hasGroundContact ~= nil then
        return p.hasGroundContact == true
    end
    if WheelContactType ~= nil and p.contact ~= nil then
        return p.contact ~= WheelContactType.NONE
    end
    return true
end

-- Lista, uma vez por veículo, os campos numéricos disponíveis no motor e nas
-- rodas. Vai para o log.txt do jogo e orienta a evolução do protocolo.
local function dumpFields(vehicle)
    local function keys(tbl)
        local out = {}
        if type(tbl) == "table" then
            for k, v in pairs(tbl) do
                local tv = type(v)
                if (tv == "number" or tv == "boolean") and type(k) == "string" then
                    table.insert(out, k)
                end
            end
        end
        table.sort(out)
        return table.concat(out, " ")
    end
    local motor = vehicle.getMotor ~= nil and vehicle:getMotor() or nil
    log("veículo: %s", tostring(vehicle.getFullName ~= nil and vehicle:getFullName() or vehicle.configFileName))
    log("motor: %s", keys(motor))
    log("spec_motorized: %s", keys(vehicle.spec_motorized))
    if vehicle.spec_motorized ~= nil then
        log("estado: motor ligado=%s, motorState=%s, pausado=%s",
            tostring(motorOn(vehicle, vehicle.spec_motorized)), tostring(vehicle.spec_motorized.motorState), tostring(isPaused()))
    end
    local spec = vehicle.spec_wheels
    if spec ~= nil and spec.wheels ~= nil and spec.wheels[1] ~= nil then
        local p = spec.wheels[1].physics
        log("wheel.physics: %s", keys(p))
        if p ~= nil then
            log("wheel.physics.netInfo: %s", keys(p.netInfo))
        end
    end
end

local function buildFrame(vehicle)
    local parts = { '{"v":1', string.format('"t":%d', math.floor(g_time or 0)) }
    local function add(key, value, fmt)
        if value == nil then
            return
        end
        if type(value) == "boolean" then
            table.insert(parts, string.format('"%s":%s', key, value and "true" or "false"))
        else
            table.insert(parts, string.format('"%s":' .. (fmt or "%.3f"), key, value))
        end
    end

    add("pz", isPaused())
    add("in", vehicle ~= nil)

    if vehicle ~= nil then
        local motorized = vehicle.spec_motorized
        if motorized ~= nil then
            local motor = vehicle.getMotor ~= nil and vehicle:getMotor() or motorized.motor
            add("on", motorOn(vehicle, motorized))
            if motor ~= nil then
                local okR, rpm = pcall(motor.getLastModulatedMotorRpm, motor)
                add("rpm", okR and num(rpm) or num(motor.lastMotorRpm), "%.1f")
                local okMin, rmin = pcall(motor.getMinRpm, motor)
                local okMax, rmax = pcall(motor.getMaxRpm, motor)
                add("rmin", okMin and num(rmin) or nil, "%.0f")
                add("rmax", okMax and num(rmax) or nil, "%.0f")
                add("clt", num(motor.manualClutchValue))
                add("gr", num(motor.gear), "%d")
                local slipTime = num(motor.clutchSlippingTime)
                local slipTimer = num(motor.clutchSlippingTimer)
                if slipTime ~= nil and slipTime > 0 and slipTimer ~= nil then
                    add("csl", math.max(0, math.min(1, slipTimer / slipTime)))
                end
                local stall = num(motor.stallTimer)
                if stall ~= nil then
                    add("stl", stall > 0)
                end
                local peak = num(motor.peakMotorTorque)
                local applied = num(motor.motorAppliedTorque)
                if peak ~= nil and peak > 0 and applied ~= nil then
                    add("trq", math.max(0, math.min(1.5, math.abs(applied) / peak)))
                end
            end
            add("ld", num(motorized.actualLoadPercentage) or num(motorized.smoothedLoadPercentage))
        end

        local drivable = vehicle.spec_drivable
        if drivable ~= nil then
            add("thr", num(drivable.axisForward))
        end
        local speed = num(vehicle.lastSpeedReal)
        if speed ~= nil then
            local dir = num(vehicle.movingDirection) or 1
            add("spd", speed * 3600 * (dir < 0 and -1 or 1), "%.2f")
        end

        local now = g_time or 0
        local frameDt = state.lastFrameTime ~= nil and (now - state.lastFrameTime) / 1000 or 0
        state.lastFrameTime = now

        local spec = vehicle.spec_wheels
        if spec ~= nil and spec.wheels ~= nil then
            local n, contact, depth, slip, slipN = 0, 0, 0, 0, 0
            local susSum, susN, water = 0, 0, 0
            local perWheel = {}
            local classes = { gRd = 0, gHd = 0, gFd = 0, gSf = 0 }
            for _, wheel in ipairs(spec.wheels) do
                local p = wheel.physics
                n = n + 1
                if hasContact(p) then
                    contact = contact + 1
                    local c = groundClass(p)
                    if c ~= nil then
                        classes[c] = classes[c] + 1
                    end
                    depth = depth + (num(p.groundDepth) or 0)
                end
                if p ~= nil and p.hasWaterContact == true then
                    water = water + 1
                end
                local len = p ~= nil and p.netInfo ~= nil and num(p.netInfo.suspensionLength) or nil
                if len ~= nil then
                    local prev = state.prevSusp[wheel]
                    if prev ~= nil and frameDt > 0 and frameDt < 0.5 then
                        local vel = (len - prev) / frameDt
                        susSum = susSum + math.abs(vel)
                        susN = susN + 1
                        table.insert(perWheel, { v = vel, x = num(p.positionX) or 0, z = num(p.positionZ) or 0 })
                    end
                    state.prevSusp[wheel] = len
                end
                local s = p ~= nil and p.netInfo ~= nil and num(p.netInfo.slip) or nil
                if s ~= nil then
                    slip = slip + math.abs(s)
                    slipN = slipN + 1
                end
            end
            add("wn", n, "%d")
            add("wc", contact, "%d")
            for k, c in pairs(classes) do
                add(k, c, "%d")
            end
            if contact > 0 then
                add("dep", depth / contact)
            end
            if slipN > 0 then
                add("slp", slip / slipN)
            end
            if susN > 0 then
                add("sus", math.min(10, susSum / susN))
            end
            -- Suspensão do eixo dianteiro (o que esterça), por lado, com sinal:
            -- uma roda subindo num obstáculo e a outra não dá o tranco no volante.
            if #perWheel >= 2 then
                local zSum = 0
                for _, w in ipairs(perWheel) do
                    zSum = zSum + w.z
                end
                local zMid = zSum / #perWheel
                local l, ln, r, rn = 0, 0, 0, 0
                for _, w in ipairs(perWheel) do
                    if w.z >= zMid then
                        if w.x >= 0 then
                            l, ln = l + w.v, ln + 1
                        else
                            r, rn = r + w.v, rn + 1
                        end
                    end
                end
                if ln > 0 and rn > 0 then
                    add("sfl", math.max(-5, math.min(5, l / ln)))
                    add("sfr", math.max(-5, math.min(5, r / rn)))
                end
            end
            add("wat", water, "%d")
        end
    end

    return table.concat(parts, ",") .. "}"
end

function PXNV99Telemetry:loadMap(name)
    log("carregado; aguardando o PXN V99 Control Center em %s", PIPE)
end

function PXNV99Telemetry:deleteMap()
    closePipe()
end

function PXNV99Telemetry:update(dt)
    if state.file == nil then
        state.sinceRetry = state.sinceRetry + dt
        if state.sinceRetry < RETRY_INTERVAL_MS then
            return
        end
        state.sinceRetry = 0
        if not openPipe() then
            return
        end
    else
        state.sinceOpen = state.sinceOpen + dt
        if state.sinceOpen >= REOPEN_INTERVAL_MS then
            closePipe()
            if not openPipe() then
                state.online = false
                state.sinceRetry = 0
                log("app não respondeu à reabertura; tentando reconectar a cada %d s", RETRY_INTERVAL_MS / 1000)
                return
            end
        end
    end

    local vehicle = currentVehicle()
    state.sinceSend = state.sinceSend + dt
    local interval = vehicle ~= nil and SEND_INTERVAL_MS or IDLE_INTERVAL_MS
    if state.sinceSend < interval then
        return
    end
    state.sinceSend = 0

    if vehicle ~= nil and state.dumpedVehicle ~= vehicle then
        state.dumpedVehicle = vehicle
        pcall(dumpFields, vehicle)
    end

    local ok, line = pcall(buildFrame, vehicle)
    if ok then
        send(line)
    else
        send(string.format('{"v":1,"msg":"erro ao montar telemetria: %s"}', tostring(line):gsub('"', "'")))
    end
end

addModEventListener(PXNV99Telemetry)
