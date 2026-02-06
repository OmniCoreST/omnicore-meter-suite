import { invoke } from "@tauri-apps/api/core";

/**
 * Export data to Excel (CSV format with UTF-8 BOM for Turkish character support)
 */
export async function exportToExcel(data: Record<string, unknown>[], filename: string, columns?: { key: string; label: string }[]) {
  if (!data || data.length === 0) {
    console.warn("No data to export");
    return;
  }

  // Get column headers
  const headers = columns
    ? columns.map(c => c.label)
    : Object.keys(data[0]);

  const keys = columns
    ? columns.map(c => c.key)
    : Object.keys(data[0]);

  // Build CSV content
  const csvRows: string[] = [];

  // Add headers
  csvRows.push(headers.map(h => `"${h}"`).join(";"));

  // Add data rows
  for (const row of data) {
    const values = keys.map(key => {
      const value = row[key];
      if (value === null || value === undefined) return "";
      if (typeof value === "string") return `"${value.replace(/"/g, '""')}"`;
      return String(value);
    });
    csvRows.push(values.join(";"));
  }

  // Create CSV with UTF-8 BOM for Excel compatibility
  const BOM = "\uFEFF";
  const csvContent = BOM + csvRows.join("\n");
  const defaultName = `${filename}_${formatDateForFilename(new Date())}.csv`;

  try {
    const savedPath = await invoke("save_file_dialog", {
      content: csvContent,
      defaultName: defaultName,
    });
    console.log("File saved to:", savedPath);
  } catch (e) {
    if (e !== "cancelled") {
      console.error("Export failed:", e);
    }
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
  return `${year}-${month}-${day}_${hours}-${minutes}`;
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
  const csvRows: string[] = [];

  rows.forEach(row => {
    const cells = row.querySelectorAll("th, td");
    const rowData = Array.from(cells).map(cell => {
      const text = (cell as HTMLElement).innerText.trim();
      return `"${text.replace(/"/g, '""')}"`;
    });
    csvRows.push(rowData.join(";"));
  });

  const BOM = "\uFEFF";
  const csvContent = BOM + csvRows.join("\n");
  const defaultName = `${filename}_${formatDateForFilename(new Date())}.csv`;

  try {
    const savedPath = await invoke("save_file_dialog", {
      content: csvContent,
      defaultName: defaultName,
    });
    console.log("File saved to:", savedPath);
  } catch (e) {
    if (e !== "cancelled") {
      console.error("Export failed:", e);
    }
  }
}
