import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import * as XLSX from "xlsx";

/**
 * Export data to Excel (.xlsx format)
 */
export async function exportToExcel(data: Record<string, unknown>[], filename: string, columns?: { key: string; label: string }[]) {
  if (!data || data.length === 0) {
    console.warn("No data to export");
    return;
  }

  const headers = columns ? columns.map(c => c.label) : Object.keys(data[0]);
  const keys = columns ? columns.map(c => c.key) : Object.keys(data[0]);

  // Build rows array: [headers, ...dataRows]
  const rows = [headers];
  for (const row of data) {
    rows.push(keys.map(key => {
      const value = row[key];
      if (value === null || value === undefined) return "";
      return String(value);
    }));
  }

  // Create workbook and worksheet
  const ws = XLSX.utils.aoa_to_sheet(rows);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Data");

  // Generate xlsx binary
  const xlsxData = XLSX.write(wb, { bookType: "xlsx", type: "array" }) as ArrayBuffer;
  const defaultName = `${filename}_${formatDateForFilename(new Date())}.xlsx`;

  try {
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });

    if (!filePath) return;

    // Write via Rust command (bypasses fs plugin scope)
    await invoke("write_export_file", {
      path: filePath,
      data: Array.from(new Uint8Array(xlsxData)),
    });
  } catch (e) {
    console.error("Export failed:", e);
    alert("Export hatası: " + e);
  }
}

/**
 * Format date for filename (YYYY-MM-DD_HH-MM)
 */
function formatDateForFilename(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");
  return `${year}-${month}-${day}_${hours}-${minutes}-${seconds}`;
}

/**
 * Export table element directly to Excel
 */
export async function exportTableToExcel(tableId: string, filename: string) {
  const table = document.getElementById(tableId);
  if (!table) {
    console.warn(`Table with id "${tableId}" not found`);
    return;
  }

  const rows = table.querySelectorAll("tr");
  const sheetData: string[][] = [];

  rows.forEach(row => {
    const cells = row.querySelectorAll("th, td");
    sheetData.push(Array.from(cells).map(cell => (cell as HTMLElement).innerText.trim()));
  });

  const ws = XLSX.utils.aoa_to_sheet(sheetData);
  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, "Data");

  const xlsxData = XLSX.write(wb, { bookType: "xlsx", type: "array" }) as ArrayBuffer;
  const defaultName = `${filename}_${formatDateForFilename(new Date())}.xlsx`;

  try {
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "Excel", extensions: ["xlsx"] }],
    });

    if (!filePath) return;

    await invoke("write_export_file", {
      path: filePath,
      data: Array.from(new Uint8Array(xlsxData)),
    });
  } catch (e) {
    console.error("Export failed:", e);
    alert("Export hatası: " + e);
  }
}
