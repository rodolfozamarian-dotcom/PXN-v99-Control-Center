# PXN V99 - sonda HID somente leitura.
# Le descritores (capabilities) e amostra relatorios de ENTRADA. Nao envia nenhum
# Output/Feature report ao volante (apenas GET de feature opcional com -ReadFeatures).
param(
    [string]$VidPid = 'VID_11FF&PID_3245',
    [int]$SampleMs = 1500,
    [switch]$ReadFeatures
)

$src = @'
using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Microsoft.Win32.SafeHandles;

public static class HidProbe
{
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern SafeFileHandle CreateFile(string name, uint access, uint share, IntPtr sa, uint disp, uint flags, IntPtr tmpl);
    [DllImport("hid.dll")] static extern bool HidD_GetPreparsedData(SafeFileHandle h, out IntPtr pp);
    [DllImport("hid.dll")] static extern bool HidD_FreePreparsedData(IntPtr pp);
    [DllImport("hid.dll")] static extern int HidP_GetCaps(IntPtr pp, byte[] caps);
    [DllImport("hid.dll")] static extern int HidP_GetValueCaps(int type, byte[] buf, ref ushort len, IntPtr pp);
    [DllImport("hid.dll")] static extern int HidP_GetButtonCaps(int type, byte[] buf, ref ushort len, IntPtr pp);
    [DllImport("hid.dll", SetLastError = true)] static extern bool HidD_GetFeature(SafeFileHandle h, byte[] buf, int len);
    [DllImport("hid.dll", CharSet = CharSet.Unicode)] static extern bool HidD_GetProductString(SafeFileHandle h, StringBuilder sb, int len);
    [DllImport("hid.dll", CharSet = CharSet.Unicode)] static extern bool HidD_GetManufacturerString(SafeFileHandle h, StringBuilder sb, int len);
    [DllImport("hid.dll", CharSet = CharSet.Unicode)] static extern bool HidD_GetSerialNumberString(SafeFileHandle h, StringBuilder sb, int len);
    [DllImport("hid.dll")] static extern bool HidD_GetAttributes(SafeFileHandle h, byte[] attr);

    static ushort U16(byte[] b, int o) { return BitConverter.ToUInt16(b, o); }
    static int I32(byte[] b, int o) { return BitConverter.ToInt32(b, o); }

    static readonly Dictionary<int, string> Pid = new Dictionary<int, string> {
        {0x21,"SetEffectReport"},{0x22,"EffectBlockIndex"},{0x23,"ParamBlockOffset"},{0x24,"RomFlag"},{0x25,"EffectType"},
        {0x26,"ET_Constant"},{0x27,"ET_Ramp"},{0x28,"ET_Custom"},{0x30,"ET_Square"},{0x31,"ET_Sine"},{0x32,"ET_Triangle"},
        {0x33,"ET_SawUp"},{0x34,"ET_SawDown"},{0x40,"ET_Spring"},{0x41,"ET_Damper"},{0x42,"ET_Inertia"},{0x43,"ET_Friction"},
        {0x50,"Duration"},{0x51,"SamplePeriod"},{0x52,"Gain"},{0x53,"TriggerButton"},{0x54,"TriggerRepeat"},{0x55,"AxesEnable"},
        {0x56,"DirectionEnable"},{0x57,"Direction"},{0x58,"TypeSpecificBlockOffset"},{0x5A,"SetEnvelopeReport"},{0x5B,"AttackLevel"},
        {0x5C,"AttackTime"},{0x5D,"FadeLevel"},{0x5E,"FadeTime"},{0x5F,"SetConditionReport"},{0x60,"CP_Offset"},
        {0x61,"PositiveCoefficient"},{0x62,"NegativeCoefficient"},{0x63,"PositiveSaturation"},{0x64,"NegativeSaturation"},
        {0x65,"DeadBand"},{0x6E,"SetPeriodicReport"},{0x6F,"Offset"},{0x70,"Magnitude"},{0x71,"Phase"},{0x72,"Period"},
        {0x73,"SetConstantForceReport"},{0x74,"SetRampForceReport"},{0x75,"RampStart"},{0x76,"RampEnd"},{0x77,"EffectOperationReport"},
        {0x78,"EffectOperation"},{0x79,"Op_Start"},{0x7A,"Op_StartSolo"},{0x7B,"Op_Stop"},{0x7C,"LoopCount"},
        {0x7D,"DeviceGainReport"},{0x7E,"DeviceGain"},{0x7F,"PidPoolReport"},{0x80,"RamPoolSize"},{0x83,"SimultaneousEffectsMax"},
        {0x89,"SetCustomForceReport"},{0x90,"BlockFreeReport"},{0x92,"PidStateReport"},{0x94,"EffectPlaying"},
        {0x95,"DeviceControlReport"},{0x96,"DeviceControl"},{0x97,"DC_EnableActuators"},{0x98,"DC_DisableActuators"},
        {0x99,"DC_StopAllEffects"},{0x9A,"DC_Reset"},{0x9B,"DC_Pause"},{0x9C,"DC_Continue"},{0x9F,"DevicePaused"},
        {0xA0,"ActuatorsEnabled"},{0xA4,"SafetySwitch"},{0xA5,"ActuatorOverrideSwitch"},{0xA6,"ActuatorPower"},
        {0xA7,"StartDelay"},{0xA8,"BlockSize"},{0xA9,"DeviceManagedPool"},{0xAA,"SharedParameterBlocks"},
        {0xAB,"CreateNewEffectReport"},{0xAC,"RamPoolAvailable"},{0x89+0x100,""},
        {0x8B,"BlockLoadReport"},{0x8C,"BlockLoadSuccess"},{0x8D,"BlockLoadFull"},{0x8E,"BlockLoadError"},{0x8F,"BlockHandle"}
    };
    static readonly Dictionary<int, string> Gd = new Dictionary<int, string> {
        {0x30,"X"},{0x31,"Y"},{0x32,"Z"},{0x33,"Rx"},{0x34,"Ry"},{0x35,"Rz"},{0x36,"Slider"},{0x37,"Dial"},{0x38,"Wheel"},{0x39,"HatSwitch"},
        {0x04,"Joystick"},{0x05,"GamePad"},{0x01,"Pointer"}
    };
    static string Name(int page, int usage) {
        string s;
        if (page == 0x0F && Pid.TryGetValue(usage, out s)) return "PID:" + s;
        if (page == 0x01 && Gd.TryGetValue(usage, out s)) return "GD:" + s;
        if (page == 0x02) return "SIM:0x" + usage.ToString("X2");
        if (page == 0x09) return "BTN:" + usage;
        return "0x" + page.ToString("X4") + ":0x" + usage.ToString("X2");
    }

    public static string Run(string path, int sampleMs, bool readFeatures)
    {
        var o = new StringBuilder();
        var h = CreateFile(path, 0xC0000000, 3, IntPtr.Zero, 3, 0x40000000, IntPtr.Zero);
        bool rw = !h.IsInvalid;
        if (!rw) { h = CreateFile(path, 0, 3, IntPtr.Zero, 3, 0x40000000, IntPtr.Zero); }
        if (h.IsInvalid) { return "ERRO abrindo " + path + " win32=" + Marshal.GetLastWin32Error(); }
        o.AppendLine("Path: " + path);
        o.AppendLine("Acesso leitura/escrita compartilhado: " + (rw ? "SIM" : "NAO (apenas metadados)"));
        var sb = new StringBuilder(256);
        if (HidD_GetManufacturerString(h, sb, 512)) o.AppendLine("Manufacturer: " + sb);
        if (HidD_GetProductString(h, sb, 512)) o.AppendLine("Product: " + sb);
        if (HidD_GetSerialNumberString(h, sb, 512)) o.AppendLine("Serial: " + sb);
        var attr = new byte[12]; attr[0] = 12;
        if (HidD_GetAttributes(h, attr)) o.AppendLine(String.Format("VID={0:X4} PID={1:X4} bcdDevice={2:X4}", U16(attr,4), U16(attr,6), U16(attr,8)));

        IntPtr pp;
        if (!HidD_GetPreparsedData(h, out pp)) return o + "ERRO preparsed";
        var caps = new byte[64];
        HidP_GetCaps(pp, caps);
        int inLen = U16(caps,4), outLen = U16(caps,6), featLen = U16(caps,8);
        o.AppendLine(String.Format("TopLevel usage={0}  InputLen={1} OutputLen={2} FeatureLen={3}", Name(U16(caps,2), U16(caps,0)), inLen, outLen, featLen));
        // offsets: 10 + 17*2 = 44 -> NumberLinkCollectionNodes
        int[] nBtn = { U16(caps,46), U16(caps,52), U16(caps,58) };
        int[] nVal = { U16(caps,48), U16(caps,54), U16(caps,60) };
        string[] tn = { "INPUT", "OUTPUT", "FEATURE" };
        var featureIds = new SortedSet<int>();
        for (int t = 0; t < 3; t++)
        {
            o.AppendLine(); o.AppendLine("=== " + tn[t] + " ===");
            var lines = new List<KeyValuePair<int,string>>();
            if (nBtn[t] > 0) {
                ushort n = (ushort)nBtn[t]; var b = new byte[72 * n];
                HidP_GetButtonCaps(t, b, ref n, pp);
                for (int i = 0; i < n; i++) { int p = i * 72;
                    int page = U16(b,p), rid = b[p+2], lu = U16(b,p+8), lup = U16(b,p+10); bool range = b[p+12] != 0;
                    int cnt = U16(b,p+16); int umin = U16(b,p+56), umax = U16(b,p+58);
                    if (t == 2) featureIds.Add(rid);
                    lines.Add(new KeyValuePair<int,string>(rid, String.Format("  RID {0,2} BTN  {1,-28} {2}  [col {3}]", rid,
                        range ? Name(page,umin) + ".." + (page==9? umax.ToString() : Name(page,umax)) : Name(page,umin), "count=" + cnt, Name(lup,lu))));
                }
            }
            if (nVal[t] > 0) {
                ushort n = (ushort)nVal[t]; var b = new byte[72 * n];
                HidP_GetValueCaps(t, b, ref n, pp);
                for (int i = 0; i < n; i++) { int p = i * 72;
                    int page = U16(b,p), rid = b[p+2], lu = U16(b,p+8), lup = U16(b,p+10); bool range = b[p+12] != 0;
                    int bits = U16(b,p+18), cnt = U16(b,p+20); int lmin = I32(b,p+40), lmax = I32(b,p+44), pmin = I32(b,p+48), pmax = I32(b,p+52);
                    int umin = U16(b,p+56), umax = U16(b,p+58);
                    if (t == 2) featureIds.Add(rid);
                    lines.Add(new KeyValuePair<int,string>(rid, String.Format("  RID {0,2} VAL  {1,-28} bits={2,-2} n={3} log[{4}..{5}] phys[{6}..{7}]  [col {8}]", rid,
                        range ? Name(page,umin) + ".." + Name(page,umax) : Name(page,umin), bits, cnt, lmin, lmax, pmin, pmax, Name(lup,lu))));
                }
            }
            lines.Sort((a, c) => a.Key.CompareTo(c.Key));
            foreach (var l in lines) o.AppendLine(l.Value);
            if (lines.Count == 0) o.AppendLine("  (nenhum)");
        }

        if (readFeatures && rw && featLen > 0) {
            o.AppendLine(); o.AppendLine("=== FEATURE GET (somente leitura) ===");
            foreach (int id in featureIds) {
                var fb = new byte[featLen]; fb[0] = (byte)id;
                bool ok = HidD_GetFeature(h, fb, fb.Length);
                o.AppendLine(String.Format("  RID {0,2}: {1}", id, ok ? BitConverter.ToString(fb) : "falhou win32=" + Marshal.GetLastWin32Error()));
            }
        }

        if (rw && inLen > 0 && sampleMs > 0) {
            o.AppendLine(); o.AppendLine("=== AMOSTRA DE INPUT REPORTS (" + sampleMs + " ms) ===");
            var seen = new Dictionary<string,int>(); var order = new List<string>(); int total = 0;
            using (var fs = new FileStream(h, FileAccess.Read, inLen, true)) {
                var sw = System.Diagnostics.Stopwatch.StartNew();
                var buf = new byte[inLen];
                while (sw.ElapsedMilliseconds < sampleMs) {
                    Task<int> t = fs.ReadAsync(buf, 0, inLen);
                    if (!t.Wait(Math.Max(1, sampleMs - (int)sw.ElapsedMilliseconds))) break;
                    total++;
                    string hex = BitConverter.ToString(buf, 0, t.Result);
                    if (!seen.ContainsKey(hex)) { seen[hex] = 0; if (order.Count < 12) order.Add(hex); }
                    seen[hex]++;
                }
                o.AppendLine(String.Format("  {0} relatorios em {1} ms  (~{2:F0} Hz), {3} distintos", total, sw.ElapsedMilliseconds, total * 1000.0 / Math.Max(1, sw.ElapsedMilliseconds), seen.Count));
                foreach (var k in order) o.AppendLine("  x" + seen[k].ToString().PadRight(5) + k);
            }
        }
        HidD_FreePreparsedData(pp);
        return o.ToString();
    }
}
'@
Add-Type -TypeDefinition $src -Language CSharp

$devs = Get-PnpDevice -PresentOnly -Class HIDClass | Where-Object { $_.InstanceId -like "HID\$VidPid*" }
if (-not $devs) { "Nenhum dispositivo HID $VidPid presente."; return }
foreach ($d in $devs) {
    $path = '\\?\' + ($d.InstanceId -replace '\\', '#').ToLower() + '#{4d1e55b2-f16f-11cf-88cb-001111000030}'
    "##################################################################"
    "# $($d.FriendlyName)  ($($d.InstanceId))"
    [HidProbe]::Run($path, $SampleMs, [bool]$ReadFeatures)
}
