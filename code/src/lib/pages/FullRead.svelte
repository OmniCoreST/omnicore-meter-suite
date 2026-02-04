<script lang="ts">
  import Icon from "$lib/components/common/Icon.svelte";
  import { t, meterStore } from "$lib/stores";

  let activeTab = $state("current");

  const tabs = [
    { id: "current", labelKey: "currentData", icon: "electric_meter" },
    { id: "historical", labelKey: "historicalData", icon: "history" },
    { id: "warnings", labelKey: "warnings", icon: "warning" },
    { id: "outages", labelKey: "outageRecords", icon: "power_off" },
  ];

  function formatNumber(value: number | undefined, decimals: number = 3): string {
    if (value === undefined) return "-";
    return value.toLocaleString("tr-TR", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  }

  // Parse monthly historical data from raw OBIS codes
  let monthlyData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.rawData) return [];

    const months = [];
    const monthNames = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran",
                        "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"];

    // Parse monthly registers (1.8.x*1 through 1.8.x*12)
    for (let month = 1; month <= 12; month++) {
      const t1Match = data.rawData.match(new RegExp(`1\\.8\\.1\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t2Match = data.rawData.match(new RegExp(`1\\.8\\.2\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t3Match = data.rawData.match(new RegExp(`1\\.8\\.3\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const t4Match = data.rawData.match(new RegExp(`1\\.8\\.4\\*${month}\\(([\\d.]+)\\*kWh\\)`));
      const totalMatch = data.rawData.match(new RegExp(`1\\.8\\.0\\*${month}\\(([\\d.]+)\\*kWh\\)`));

      months.push({
        month,
        monthName: monthNames[month - 1],
        t1: t1Match ? parseFloat(t1Match[1]) : 0,
        t2: t2Match ? parseFloat(t2Match[1]) : 0,
        t3: t3Match ? parseFloat(t3Match[1]) : 0,
        t4: t4Match ? parseFloat(t4Match[1]) : 0,
        total: totalMatch ? parseFloat(totalMatch[1]) : 0,
      });
    }

    return months;
  });

  // Parse warnings data from raw OBIS codes
  let warningsData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.rawData) return { voltage: [], current: [], magnetic: [] };

    const parseWarningRecords = (baseCode: string, count: number) => {
      const records = [];
      for (let i = 1; i <= count; i++) {
        const match = data.rawData.match(
          new RegExp(`${baseCode.replace('.', '\\.')}\\*${i}\\(([^;]+);([^)]+)\\)`)
        );
        if (match) {
          records.push({
            id: i,
            start: match[1],
            end: match[2],
          });
        }
      }
      return records;
    };

    // Voltage warnings (96.77.4*1-10)
    const voltageCount = data.rawData.match(/96\.7\.4\((\d+)\)/);
    const voltage = parseWarningRecords('96.77.4', 10);

    // Current warnings (96.77.5*1-10)
    const currentCount = data.rawData.match(/96\.7\.5\((\d+)\)/);
    const current = parseWarningRecords('96.77.5', 10);

    // Magnetic field warnings (96.77.6*1-10)
    const magneticCount = data.rawData.match(/96\.7\.6\((\d+)\)/);
    const magnetic = parseWarningRecords('96.77.6', 10);

    return {
      voltage: { count: voltageCount ? parseInt(voltageCount[1]) : 0, records: voltage },
      current: { count: currentCount ? parseInt(currentCount[1]) : 0, records: current },
      magnetic: { count: magneticCount ? parseInt(magneticCount[1]) : 0, records: magnetic },
    };
  });

  // Parse outage records from raw OBIS codes
  let outagesData = $derived.by(() => {
    const data = $meterStore.shortReadData;
    if (!data || !data.rawData) return {};

    const parseOutageRecords = (baseCode: string, maxCount: number) => {
      const records = [];
      for (let i = 1; i <= maxCount; i++) {
        const match = data.rawData.match(
          new RegExp(`${baseCode.replace(/\./g, '\\.')}\\*${i}\\(([^;]+);([^)]+)\\)`)
        );
        if (match) {
          const start = match[1];
          const end = match[2];
          // Calculate duration in minutes
          const startDate = parseDateTime(start);
          const endDate = parseDateTime(end);
          const durationMs = endDate.getTime() - startDate.getTime();
          const durationMin = Math.floor(durationMs / 60000);

          records.push({
            id: i,
            start,
            end,
            duration: durationMin,
          });
        }
      }
      return records;
    };

    // Three-phase outages
    const threePhaseLongCount = data.rawData.match(/96\.7\.0\((\d+)\)/);
    const threePhaseShortCount = data.rawData.match(/96\.7\.00\((\d+)\)/);

    // Phase 1 outages
    const phase1LongCount = data.rawData.match(/96\.7\.1\((\d+)\)/);
    const phase1ShortCount = data.rawData.match(/96\.7\.10\((\d+)\)/);

    // Phase 2 outages
    const phase2LongCount = data.rawData.match(/96\.7\.2\((\d+)\)/);
    const phase2ShortCount = data.rawData.match(/96\.7\.20\((\d+)\)/);

    // Phase 3 outages
    const phase3LongCount = data.rawData.match(/96\.7\.3\((\d+)\)/);
    const phase3ShortCount = data.rawData.match(/96\.7\.30\((\d+)\)/);

    return {
      threePhase: {
        long: { count: threePhaseLongCount ? parseInt(threePhaseLongCount[1]) : 0, records: parseOutageRecords('96.77.0', 99) },
        short: { count: threePhaseShortCount ? parseInt(threePhaseShortCount[1]) : 0, records: parseOutageRecords('96.77.00', 99) },
      },
      phase1: {
        long: { count: phase1LongCount ? parseInt(phase1LongCount[1]) : 0, records: parseOutageRecords('96.77.1', 99) },
        short: { count: phase1ShortCount ? parseInt(phase1ShortCount[1]) : 0, records: parseOutageRecords('96.77.10', 99) },
      },
      phase2: {
        long: { count: phase2LongCount ? parseInt(phase2LongCount[1]) : 0, records: parseOutageRecords('96.77.2', 99) },
        short: { count: phase2ShortCount ? parseInt(phase2ShortCount[1]) : 0, records: parseOutageRecords('96.77.20', 99) },
      },
      phase3: {
        long: { count: phase3LongCount ? parseInt(phase3LongCount[1]) : 0, records: parseOutageRecords('96.77.3', 99) },
        short: { count: phase3ShortCount ? parseInt(phase3ShortCount[1]) : 0, records: parseOutageRecords('96.77.30', 99) },
      },
    };
  });

  function parseDateTime(dateTimeStr: string): Date {
    // Format: yy-mm-dd,hh:mm
    const [datePart, timePart] = dateTimeStr.split(',');
    const [year, month, day] = datePart.split('-').map(Number);
    const [hour, minute] = timePart.split(':').map(Number);
    return new Date(2000 + year, month - 1, day, hour, minute);
  }

  function formatDuration(minutes: number): string {
    if (minutes < 60) return `${minutes} dk`;
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    return `${hours} saat ${mins} dk`;
  }
</script>

<div class="space-y-6">
  <!-- Header -->
  <div
    class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-6 shadow-sm"
  >
    <div class="flex items-start justify-between">
      <div>
        <h3 class="text-xl font-bold text-slate-900 dark:text-white mb-2">{$t.fullRead}</h3>
        <p class="text-sm text-slate-500 dark:text-slate-400">{$t.fullReadDescription}</p>
      </div>
    </div>
  </div>

  {#if $meterStore.shortReadData}
    {@const data = $meterStore.shortReadData}

    <!-- Tabs -->
    <div class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl shadow-sm overflow-hidden">
      <div class="flex border-b border-slate-200 dark:border-[#334a5e]">
        {#each tabs as tab}
          <button
            onclick={() => activeTab = tab.id}
            class="flex-1 flex items-center justify-center gap-2 px-6 py-4 font-medium transition-colors
              {activeTab === tab.id
                ? 'bg-primary/10 text-primary border-b-2 border-primary'
                : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-[#1a2632]'}"
          >
            <Icon name={tab.icon} size="sm" />
            <span>{$t[tab.labelKey]}</span>
          </button>
        {/each}
      </div>

      <div class="p-6">
        {#if activeTab === "current"}
          <!-- Current Data Tab - Same as Short Read -->
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <!-- Show all current meter values -->
            <div class="space-y-4">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="badge" class="text-primary" size="sm" />
                {$t.meterIdentity}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.serialNumber}</span>
                  <span class="font-mono font-bold">{data.serialNumber || "510095849"}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.programVersion}</span>
                  <span class="font-mono">{data.programVersion}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.meterDate}</span>
                  <span class="font-mono">{data.meterDate} {data.meterTime}</span>
                </div>
              </div>
            </div>

            <div class="space-y-4">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="bolt" class="text-primary" size="sm" />
                {$t.activeEnergyImport}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.total}</span>
                  <span class="font-mono font-bold">{formatNumber(data.activeEnergyImportTotal)} kWh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.t1Day}</span>
                  <span class="font-mono">{formatNumber(data.activeEnergyImportT1)} kWh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.t2Peak}</span>
                  <span class="font-mono">{formatNumber(data.activeEnergyImportT2)} kWh</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.t3Night}</span>
                  <span class="font-mono">{formatNumber(data.activeEnergyImportT3)} kWh</span>
                </div>
              </div>
            </div>

            <div class="space-y-4">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="electric_bolt" class="text-primary" size="sm" />
                {$t.instantaneousValues}
              </h4>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.voltageL1}</span>
                  <span class="font-mono font-bold">{formatNumber(data.voltageL1, 1)} V</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-slate-500">{$t.frequency}</span>
                  <span class="font-mono">{formatNumber(data.frequency, 1)} Hz</span>
                </div>
              </div>
            </div>
          </div>
        {:else if activeTab === "historical"}
          <!-- Historical Data Tab -->
          <div class="space-y-6">
            <div class="flex items-center justify-between">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                <Icon name="calendar_month" class="text-primary" />
                12 Aylık Enerji Geçmişi
              </h4>
              <span class="text-sm text-slate-500">Son 12 ay</span>
            </div>

            <!-- Monthly Data Table -->
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead class="bg-slate-50 dark:bg-[#111c26]">
                  <tr class="border-b border-slate-200 dark:border-[#334a5e]">
                    <th class="px-4 py-3 text-left font-bold text-slate-700 dark:text-slate-300">Ay</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T1 Gündüz</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T2 Puant</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T3 Gece</th>
                    <th class="px-4 py-3 text-right font-bold text-slate-700 dark:text-slate-300">T4</th>
                    <th class="px-4 py-3 text-right font-bold text-primary">Toplam</th>
                  </tr>
                </thead>
                <tbody>
                  {#each monthlyData as monthData, i}
                    <tr class="border-b border-slate-100 dark:border-[#334a5e]/30 hover:bg-slate-50 dark:hover:bg-[#1a2632] transition-colors">
                      <td class="px-4 py-3">
                        <div class="flex items-center gap-2">
                          <div class="size-2 rounded-full bg-primary"></div>
                          <span class="font-medium text-slate-900 dark:text-white">{monthData.monthName}</span>
                        </div>
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t1)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t2)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t3)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono text-slate-600 dark:text-slate-400">
                        {formatNumber(monthData.t4)} kWh
                      </td>
                      <td class="px-4 py-3 text-right font-mono font-bold text-primary">
                        {formatNumber(monthData.total)} kWh
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>

            <!-- Summary Cards -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mt-6">
              {#each ["T1", "T2", "T3", "Total"] as tariff, i}
                {@const totalKey = i === 0 ? 't1' : i === 1 ? 't2' : i === 2 ? 't3' : 'total'}
                {@const sum = monthlyData.reduce((acc, m) => acc + m[totalKey], 0)}
                <div class="bg-gradient-to-br from-primary/10 to-emerald-500/10 dark:from-primary/20 dark:to-emerald-500/20 rounded-xl p-4 border border-primary/20">
                  <div class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider mb-1">
                    {tariff === "Total" ? "Yıllık Toplam" : `${tariff} Toplamı`}
                  </div>
                  <div class="text-2xl font-bold text-primary">
                    {formatNumber(sum, 0)}
                  </div>
                  <div class="text-xs text-slate-600 dark:text-slate-400 mt-1">kWh</div>
                </div>
              {/each}
            </div>
          </div>
        {:else if activeTab === "warnings"}
          <!-- Warnings Tab -->
          <div class="space-y-6">
            <!-- Voltage Warnings -->
            <div class="bg-gradient-to-br from-amber-50 to-orange-50 dark:from-amber-900/20 dark:to-orange-900/20 rounded-xl p-6 border border-amber-200 dark:border-amber-800">
              <div class="flex items-center justify-between mb-4">
                <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                  <Icon name="flash_on" class="text-amber-500" />
                  Gerilim Uyarıları
                </h4>
                <span class="px-3 py-1 bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 rounded-full text-xs font-bold">
                  Toplam: {warningsData.voltage.count}
                </span>
              </div>

              {#if warningsData.voltage.records.length > 0}
                <div class="overflow-x-auto">
                  <table class="w-full text-sm">
                    <thead class="bg-amber-100/50 dark:bg-amber-900/30">
                      <tr class="border-b border-amber-200 dark:border-amber-800">
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Başlangıç</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Bitiş</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Açıklama</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each warningsData.voltage.records as record}
                        <tr class="border-b border-amber-100 dark:border-amber-900/30 hover:bg-amber-50 dark:hover:bg-amber-900/20 transition-colors">
                          <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                          <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                          <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                          <td class="px-4 py-2 text-slate-600 dark:text-slate-400">Faz sırası/polarite hatası</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {:else}
                <p class="text-center text-slate-500 py-4">Gerilim uyarısı kaydı yok</p>
              {/if}
            </div>

            <!-- Current Warnings -->
            <div class="bg-gradient-to-br from-blue-50 to-cyan-50 dark:from-blue-900/20 dark:to-cyan-900/20 rounded-xl p-6 border border-blue-200 dark:border-blue-800">
              <div class="flex items-center justify-between mb-4">
                <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                  <Icon name="bolt" class="text-blue-500" />
                  Akım Uyarıları
                </h4>
                <span class="px-3 py-1 bg-blue-500/10 border border-blue-500/20 text-blue-600 dark:text-blue-400 rounded-full text-xs font-bold">
                  Toplam: {warningsData.current.count}
                </span>
              </div>

              {#if warningsData.current.records.length > 0}
                <div class="overflow-x-auto">
                  <table class="w-full text-sm">
                    <thead class="bg-blue-100/50 dark:bg-blue-900/30">
                      <tr class="border-b border-blue-200 dark:border-blue-800">
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Başlangıç</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Bitiş</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Açıklama</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each warningsData.current.records as record}
                        <tr class="border-b border-blue-100 dark:border-blue-900/30 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors">
                          <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                          <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                          <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                          <td class="px-4 py-2 text-slate-600 dark:text-slate-400">Akım bağlantı hatası/polarite tersliği</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {:else}
                <p class="text-center text-slate-500 py-4">Akım uyarısı kaydı yok</p>
              {/if}
            </div>

            <!-- Magnetic Field Warnings -->
            <div class="bg-gradient-to-br from-red-50 to-rose-50 dark:from-red-900/20 dark:to-rose-900/20 rounded-xl p-6 border border-red-200 dark:border-red-800">
              <div class="flex items-center justify-between mb-4">
                <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2">
                  <Icon name="sensors" class="text-red-500" />
                  Manyetik Alan Uyarıları
                </h4>
                <span class="px-3 py-1 bg-red-500/10 border border-red-500/20 text-red-600 dark:text-red-400 rounded-full text-xs font-bold">
                  Toplam: {warningsData.magnetic.count}
                </span>
              </div>

              {#if warningsData.magnetic.records.length > 0}
                <div class="overflow-x-auto">
                  <table class="w-full text-sm">
                    <thead class="bg-red-100/50 dark:bg-red-900/30">
                      <tr class="border-b border-red-200 dark:border-red-800">
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">#</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Başlangıç</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Bitiş</th>
                        <th class="px-4 py-2 text-left font-bold text-slate-700 dark:text-slate-300">Açıklama</th>
                      </tr>
                    </thead>
                    <tbody>
                      {#each warningsData.magnetic.records as record}
                        <tr class="border-b border-red-100 dark:border-red-900/30 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors">
                          <td class="px-4 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                          <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                          <td class="px-4 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                          <td class="px-4 py-2 text-slate-600 dark:text-slate-400">Manyetik alan müdahalesi (&gt;400mT)</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {:else}
                <p class="text-center text-slate-500 py-4">Manyetik alan uyarısı kaydı yok</p>
              {/if}
            </div>
          </div>
        {:else if activeTab === "outages"}
          <!-- Outages Tab -->
          <div class="space-y-6">
            <!-- Three-Phase Outages -->
            <div class="bg-gradient-to-br from-slate-50 to-gray-50 dark:from-slate-900/40 dark:to-gray-900/40 rounded-xl p-6 border border-slate-200 dark:border-slate-700">
              <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2 mb-4">
                <Icon name="power_off" class="text-slate-600" />
                Üç Faz Kesintileri
              </h4>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4">
                  <div class="text-xs text-slate-500 dark:text-slate-400 mb-1">Uzun Kesinti (≥180 sn)</div>
                  <div class="text-2xl font-bold text-red-600">{outagesData.threePhase?.long.count || 0}</div>
                </div>
                <div class="bg-amber-500/10 border border-amber-500/20 rounded-lg p-4">
                  <div class="text-xs text-slate-500 dark:text-slate-400 mb-1">Kısa Kesinti (&lt;180 sn)</div>
                  <div class="text-2xl font-bold text-amber-600">{outagesData.threePhase?.short.count || 0}</div>
                </div>
              </div>

              {#if outagesData.threePhase?.long.records.length > 0 || outagesData.threePhase?.short.records.length > 0}
                <div class="space-y-4">
                  {#if outagesData.threePhase.long.records.length > 0}
                    <div>
                      <h5 class="text-sm font-bold text-slate-700 dark:text-slate-300 mb-2">Uzun Kesintiler</h5>
                      <div class="overflow-x-auto">
                        <table class="w-full text-sm">
                          <thead class="bg-slate-100 dark:bg-slate-800">
                            <tr class="border-b border-slate-200 dark:border-slate-700">
                              <th class="px-3 py-2 text-left font-bold text-slate-600 dark:text-slate-400">#</th>
                              <th class="px-3 py-2 text-left font-bold text-slate-600 dark:text-slate-400">Başlangıç</th>
                              <th class="px-3 py-2 text-left font-bold text-slate-600 dark:text-slate-400">Bitiş</th>
                              <th class="px-3 py-2 text-right font-bold text-slate-600 dark:text-slate-400">Süre</th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each outagesData.threePhase.long.records.slice(0, 10) as record}
                              <tr class="border-b border-slate-100 dark:border-slate-800 hover:bg-slate-50 dark:hover:bg-slate-800/50">
                                <td class="px-3 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                                <td class="px-3 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                                <td class="px-3 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                                <td class="px-3 py-2 text-right font-mono text-red-600 dark:text-red-400">{formatDuration(record.duration)}</td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  {/if}

                  {#if outagesData.threePhase.short.records.length > 0}
                    <div>
                      <h5 class="text-sm font-bold text-slate-700 dark:text-slate-300 mb-2">Kısa Kesintiler</h5>
                      <div class="overflow-x-auto">
                        <table class="w-full text-sm">
                          <thead class="bg-slate-100 dark:bg-slate-800">
                            <tr class="border-b border-slate-200 dark:border-slate-700">
                              <th class="px-3 py-2 text-left font-bold text-slate-600 dark:text-slate-400">#</th>
                              <th class="px-3 py-2 text-left font-bold text-slate-600 dark:text-slate-400">Başlangıç</th>
                              <th class="px-3 py-2 text-left font-bold text-slate-600 dark:text-slate-400">Bitiş</th>
                              <th class="px-3 py-2 text-right font-bold text-slate-600 dark:text-slate-400">Süre</th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each outagesData.threePhase.short.records.slice(0, 10) as record}
                              <tr class="border-b border-slate-100 dark:border-slate-800 hover:bg-slate-50 dark:hover:bg-slate-800/50">
                                <td class="px-3 py-2 font-mono text-slate-600 dark:text-slate-400">{record.id}</td>
                                <td class="px-3 py-2 font-mono text-slate-900 dark:text-white">{record.start}</td>
                                <td class="px-3 py-2 font-mono text-slate-900 dark:text-white">{record.end}</td>
                                <td class="px-3 py-2 text-right font-mono text-amber-600 dark:text-amber-400">{formatDuration(record.duration)}</td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  {/if}
                </div>
              {:else}
                <p class="text-center text-slate-500 py-4">Üç faz kesinti kaydı yok</p>
              {/if}
            </div>

            <!-- Per-Phase Outages -->
            {#each [
              { phase: "1. Faz (L1)", icon: "looks_one", data: outagesData.phase1, color: "emerald" },
              { phase: "2. Faz (L2)", icon: "looks_two", data: outagesData.phase2, color: "sky" },
              { phase: "3. Faz (L3)", icon: "looks_3", data: outagesData.phase3, color: "violet" }
            ] as phaseInfo}
              {#if phaseInfo.data?.long.count > 0 || phaseInfo.data?.short.count > 0}
                <div class="bg-gradient-to-br from-{phaseInfo.color}-50 to-{phaseInfo.color}-100/50 dark:from-{phaseInfo.color}-900/20 dark:to-{phaseInfo.color}-900/10 rounded-xl p-6 border border-{phaseInfo.color}-200 dark:border-{phaseInfo.color}-800">
                  <h4 class="font-bold text-slate-900 dark:text-white flex items-center gap-2 mb-4">
                    <Icon name={phaseInfo.icon} class="text-{phaseInfo.color}-500" />
                    {phaseInfo.phase} Kesintileri
                  </h4>

                  <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                    <div class="bg-red-500/10 border border-red-500/20 rounded-lg p-3">
                      <div class="text-xs text-slate-500 dark:text-slate-400 mb-1">Uzun Kesinti</div>
                      <div class="text-xl font-bold text-red-600">{phaseInfo.data?.long.count || 0}</div>
                    </div>
                    <div class="bg-amber-500/10 border border-amber-500/20 rounded-lg p-3">
                      <div class="text-xs text-slate-500 dark:text-slate-400 mb-1">Kısa Kesinti</div>
                      <div class="text-xl font-bold text-amber-600">{phaseInfo.data?.short.count || 0}</div>
                    </div>
                  </div>

                  {#if phaseInfo.data.long.records.length > 0}
                    <details class="mb-3">
                      <summary class="cursor-pointer text-sm font-bold text-slate-700 dark:text-slate-300 hover:text-primary">
                        Uzun Kesintiler ({phaseInfo.data.long.records.length})
                      </summary>
                      <div class="overflow-x-auto mt-2">
                        <table class="w-full text-xs">
                          <thead class="bg-slate-100 dark:bg-slate-800">
                            <tr class="border-b border-slate-200 dark:border-slate-700">
                              <th class="px-2 py-1 text-left">#</th>
                              <th class="px-2 py-1 text-left">Başlangıç</th>
                              <th class="px-2 py-1 text-left">Bitiş</th>
                              <th class="px-2 py-1 text-right">Süre</th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each phaseInfo.data.long.records.slice(0, 10) as record}
                              <tr class="border-b border-slate-100 dark:border-slate-800">
                                <td class="px-2 py-1 font-mono">{record.id}</td>
                                <td class="px-2 py-1 font-mono">{record.start}</td>
                                <td class="px-2 py-1 font-mono">{record.end}</td>
                                <td class="px-2 py-1 text-right font-mono text-red-600">{formatDuration(record.duration)}</td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      </div>
                    </details>
                  {/if}

                  {#if phaseInfo.data.short.records.length > 0}
                    <details>
                      <summary class="cursor-pointer text-sm font-bold text-slate-700 dark:text-slate-300 hover:text-primary">
                        Kısa Kesintiler ({phaseInfo.data.short.records.length})
                      </summary>
                      <div class="overflow-x-auto mt-2">
                        <table class="w-full text-xs">
                          <thead class="bg-slate-100 dark:bg-slate-800">
                            <tr class="border-b border-slate-200 dark:border-slate-700">
                              <th class="px-2 py-1 text-left">#</th>
                              <th class="px-2 py-1 text-left">Başlangıç</th>
                              <th class="px-2 py-1 text-left">Bitiş</th>
                              <th class="px-2 py-1 text-right">Süre</th>
                            </tr>
                          </thead>
                          <tbody>
                            {#each phaseInfo.data.short.records.slice(0, 10) as record}
                              <tr class="border-b border-slate-100 dark:border-slate-800">
                                <td class="px-2 py-1 font-mono">{record.id}</td>
                                <td class="px-2 py-1 font-mono">{record.start}</td>
                                <td class="px-2 py-1 font-mono">{record.end}</td>
                                <td class="px-2 py-1 text-right font-mono text-amber-600">{formatDuration(record.duration)}</td>
                              </tr>
                            {/each}
                          </tbody>
                        </table>
                      </div>
                    </details>
                  {/if}
                </div>
              {/if}
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <div
      class="bg-white dark:bg-surface-dark border border-slate-200 dark:border-[#334a5e] rounded-xl p-12 shadow-sm text-center"
    >
      <Icon name="assignment" class="text-6xl text-slate-300 dark:text-slate-600 mb-4" />
      <p class="text-slate-500 dark:text-slate-400">
        {$t.noDataYet || "Connect to a meter to see full reading data"}
      </p>
    </div>
  {/if}
</div>
