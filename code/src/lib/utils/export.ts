import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import * as XLSX from "xlsx";
import html2pdf from "html2pdf.js";

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

/**
 * Export HTML content to PDF with save dialog
 *
 * Uses a fixed-width off-screen container that exactly matches A4 proportions
 * so html2canvas captures every element without clipping or overflow.
 */
export async function exportToPdf(htmlContent: string, defaultFilename?: string) {
  // A4 usable width: 210mm - 2×10mm margins = 190mm ≈ 718px at 96 DPI
  const PAGE_WIDTH_PX = 718;
  // Wrapper slightly wider so nothing clips on the right edge
  const WRAPPER_WIDTH_PX = PAGE_WIDTH_PX + 2;

  try {
    // Ask user where to save
    const defaultName = defaultFilename || `OmniCore-Rapor-${formatDateForFilename(new Date())}.pdf`;
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });

    if (!filePath) return; // User cancelled

    // Extract style and body content from full HTML document
    const styleMatch = htmlContent.match(/<style[^>]*>([\s\S]*?)<\/style>/i);
    const bodyMatch = htmlContent.match(/<body[^>]*>([\s\S]*?)<\/body>/i);
    const bodyContent = bodyMatch ? bodyMatch[1] : htmlContent;

    // Remove no-print elements (like the print button)
    const cleanContent = bodyContent.replace(/<div[^>]*class="no-print"[^>]*>[\s\S]*?<\/div>/gi, '');

    // Create off-screen wrapper – must be in DOM for html2canvas
    const wrapper = document.createElement("div");
    wrapper.style.position = "fixed";
    wrapper.style.left = "0";
    wrapper.style.top = "0";
    wrapper.style.width = WRAPPER_WIDTH_PX + "px";
    wrapper.style.zIndex = "-9999";
    wrapper.style.opacity = "0";
    wrapper.style.pointerEvents = "none";
    wrapper.style.overflow = "visible";

    const tempDiv = document.createElement("div");
    tempDiv.style.width = PAGE_WIDTH_PX + "px";
    tempDiv.style.maxWidth = PAGE_WIDTH_PX + "px";
    tempDiv.style.background = "#fff";
    tempDiv.style.color = "#1e293b";
    tempDiv.style.fontFamily = "Arial, Helvetica, sans-serif";
    tempDiv.style.fontSize = "12px";
    tempDiv.style.lineHeight = "1.5";
    tempDiv.style.padding = "0";
    tempDiv.setAttribute("data-pdf-container", "true");

    if (styleMatch) {
      const styleEl = document.createElement("style");
      styleEl.textContent = styleMatch[1];
      tempDiv.appendChild(styleEl);
    }

    const contentDiv = document.createElement("div");
    contentDiv.innerHTML = cleanContent;
    tempDiv.appendChild(contentDiv);

    wrapper.appendChild(tempDiv);
    document.body.appendChild(wrapper);

    // PDF options – onclone makes the hidden wrapper visible in the clone
    const options = {
      margin: [0.4, 0.4, 0.4, 0.4] as [number, number, number, number], // top, left, bottom, right in inches
      image: { type: 'jpeg' as const, quality: 0.98 },
      html2canvas: {
        scale: 2,
        useCORS: true,
        letterRendering: true,
        scrollX: 0,
        scrollY: 0,
        windowWidth: WRAPPER_WIDTH_PX,
        onclone: (clonedDoc: Document) => {
          const el = clonedDoc.querySelector('[data-pdf-container]') as HTMLElement;
          if (el && el.parentElement) {
            el.parentElement.style.opacity = "1";
            el.parentElement.style.position = "static";
            el.parentElement.style.zIndex = "auto";
            el.parentElement.style.overflow = "visible";
            el.parentElement.style.width = WRAPPER_WIDTH_PX + "px";
          }
        }
      },
      jsPDF: {
        unit: 'in',
        format: 'a4',
        orientation: 'portrait' as const
      },
      // Enable automatic page-breaking with sensible avoidance
      pagebreak: { mode: ['avoid-all', 'css', 'legacy'] }
    };

    // Generate PDF as blob
    const pdfBlob = await html2pdf()
      .set(options)
      .from(tempDiv)
      .outputPdf('blob');

    // Convert blob to array buffer
    const arrayBuffer = await pdfBlob.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);

    // Write to file via Rust command
    await invoke("write_export_file", {
      path: filePath,
      data: Array.from(uint8Array),
    });

    // Clean up
    document.body.removeChild(wrapper);

    return filePath;
  } catch (e) {
    console.error("PDF export failed:", e);
    alert("PDF export hatası: " + e);
    throw e;
  }
}
