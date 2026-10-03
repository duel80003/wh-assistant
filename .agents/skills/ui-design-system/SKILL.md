---
name: ui-design-system
description: >-
  UI/UX design system and visual style guide for WHassistant desktop application (Dioxus 0.7 + Tailwind CSS).
  Use this skill whenever creating, modifying, reviewing, or styling any frontend UI components, views, layouts,
  color schemes, buttons, status badges, tables, or modals to ensure strict adherence to the design specifications.
---

# WHassistant 單據管理系統 - UI/UX 設計規範 Skill

本 Skill 定義全系統統一之色彩、字體、佈局、狀態反饋與核心元件樣式標準。**所有前端（Dioxus 0.7 + Tailwind CSS）介面實作必須嚴格遵守本規範**。

完整規格文件請參考：[docs/receipt/ui_design_system.md](../../../docs/receipt/ui_design_system.md)

---

## 一、 設計風格與原則

1. **專業工控與高密度資訊流**：介面以清楚傳遞數據為核心，兼顧工單影像核對效率與財務催收提醒。
2. **現代深色為主、高對比易讀（Modern Slate Dark）**：
   - 預設採用精緻的 **Slate 灰階深色主題**（低眩光、護眼、能突顯單據圖片與彩色狀態徽章）。
   - 避免純黑（`#000000`），全面採用 Slate 系列（`slate-950`、`slate-900`、`slate-800`）營造細緻層次。
3. **語意化色彩一致性**：收費狀態、逾期警示與操作反饋在所有頁面保持絕對一致的顏色對應。
4. **數字排版對齊**：金額、工單號、日期及逾期天數一律採用等寬數字字型（`tabular-nums font-mono`），確保直欄對齊。
5. **全繁體中文介面**：所有按鈕、欄位、標籤、對話框文字一律為繁體中文，禁止英文夾雜。

---

## 二、 色彩體系 (Color Palette)

```
+-------------------------------------------------------------------------+
| 主色彩 (Primary)      : 靛藍 Indigo-600 (#4f46e5) -> 品牌、主操作、導覽列選取   |
| 次要色彩 (Secondary)  : 岩灰 Slate-700 (#334155)  -> 次級按鈕、邊框、圖表軸線   |
| 背景基底 (Surface)    : 深岩 Slate-950 (#020617)  -> 視窗底色、輸入框背景       |
| 卡片層級 (Elevation)  : 灰藍 Slate-900 (#0f172a)  -> 模組卡片、對話框、側邊欄   |
| 邊框線條 (Border)     : 邊框 Slate-800 (#1e293b)  -> 分隔線、卡片輪廓           |
+-------------------------------------------------------------------------+
```

### 1. 介面基底色彩調色盤

| 色彩角色 | Tailwind Token | HEX 代碼 | 適用場景與元件 |
| :--- | :--- | :--- | :--- |
| **主品牌色 (Primary)** | `indigo-600` | `#4f46e5` | 主要點擊按鈕【確認儲存】、啟用開關、分頁活動指示器 |
| **主色懸停 (Primary Hover)** | `indigo-500` | `#6366f1` | 主按鈕 Hover 狀態 |
| **次要輔助色 (Secondary)** | `slate-700` | `#334155` | 次級按鈕背景、表格表頭背景、非活動標籤 |
| **全域主背景 (App Background)**| `slate-950` | `#020617` | 軟體桌面視窗整體底色 |
| **面板與卡片 (Card Surface)** | `slate-900` | `#0f172a` | 左側導覽列、工作區卡片、彈窗面板 |
| **懸浮與懸停底色 (Hover Surface)**| `slate-800` | `#1e293b` | 表格列 Hover、清單選取態、次級按鈕 Hover |
| **邊框線 (Border Line)** | `slate-800` | `#1e293b` | 卡片外框、表格分隔線、表單預設邊框 |
| **聚焦高亮邊框 (Focus Ring)** | `indigo-500` | `#6366f1` | 輸入框聚焦（Focus Ring） |

### 2. 業務狀態與語意色彩 (Semantic & Status Colors)

> ⚠️ **嚴格規定**：所有頁面遇到下列業務狀態時，必須使用相同的色彩樣式，不可任意混用！

| 業務狀態 | 主題色彩 | 標籤背景與文字樣式 (Tailwind Classes) | 視覺含義 |
| :--- | :--- | :--- | :--- |
| **【未收費】** (Unpaid) | 琥珀金 (Amber) | `bg-amber-500/10 text-amber-400 border border-amber-500/30` | 提醒 attention、待處理之正常未收帳款 |
| **【已收費】** (Paid) | 翡翠綠 (Emerald) | `bg-emerald-500/10 text-emerald-400 border border-emerald-500/30` | 安全 safe、款項已結清無風險 |
| **【逾期未收】** (Overdue) | 玫瑰紅 (Rose) | `bg-rose-500/10 text-rose-400 border border-rose-500/30 font-semibold` | 警告 urgent、超過應收期限須立即催收 |
| **【待確認】** (Unconfirmed)| 鮮黃 (Yellow) | `bg-yellow-500/10 text-yellow-400 border border-yellow-500/30` | 等待人工審核單據內容 |
| **【處理中】** (Processing) | 晴空藍 (Sky) | `bg-sky-500/10 text-sky-400 border border-sky-500/30 animate-pulse` | Ollama 正在辨識中，帶微幅呼吸動態 |
| **【辨識失敗】** (Failed) | 亮紅 (Red) | `bg-red-500/10 text-red-400 border border-red-500/30` | 提取失敗、模型逾時，提示重試或手動輸入 |

---

## 三、 排版與文字體系 (Typography)

### 1. 字型家族 (Font Family)
```css
font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", "PingFang TC", "Microsoft JhengHei", sans-serif;
```
- 中文字體優先使用 macOS/iOS 的 **PingFang TC** 與 Windows 的 **Microsoft JhengHei**。
- 所有金額、單號、日期等數據，必須強制附加：`font-mono tabular-nums`，保證字距等寬且上下直行對齊。

### 2. 字級規範
- 頁面主標題 (H1)：`text-2xl font-bold text-slate-100`
- 看板重點數字 (Metric)：`text-3xl font-extrabold font-mono tracking-tight`
- 區塊標題 (H2)：`text-lg font-semibold text-slate-200`
- 主要內文 (Body)：`text-sm font-normal text-slate-300`
- 次要說明 (Caption)：`text-xs font-normal text-slate-400`

---

## 四、 版面結構與四大核心頁面 (App Layout)

整體架構採「**左側固定側邊欄 (240px `w-60`) + 右側主內容流 (`p-6`)**」：

### 1. 左側導覽列 (Sidebar)
- 容器：`w-60 bg-slate-900 border-r border-slate-800 flex flex-col justify-between p-4 shrink-0`
- 未選中項目：`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors`
- 選中項目：`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-white bg-indigo-600/90 shadow-sm shadow-indigo-500/20`
- 提醒氣泡徽章：`ml-auto px-2 py-0.5 rounded-full text-xs font-bold bg-rose-500 text-white animate-pulse`

### 2. 【待審單據】頁面 (Review Queue)
- 頂部拖曳上傳區：`border-2 border-dashed border-slate-700 hover:border-indigo-500 rounded-xl bg-slate-900/50 hover:bg-indigo-950/20 p-6 flex flex-col items-center justify-center cursor-pointer transition-all`
- 核心雙屏比例：`45% : 55%`
  - 左側圖片區（45%）：支援縮放平移預覽容器，置頂半透明工具列（放大、縮小、旋轉、還原）。
  - 右側表單區（55%）：2 欄網格表單（工單號、施工人員、施工日期、總金額、收費期限方案、應收截止日），底部操作列包含【確認儲存】、【重新辨識】、【手動補錄】、【刪除單據】。

### 3. 【歷史單據】頁面 (History Archive)
- 頂部搜尋篩選列：搜尋輸入框、收費狀態下拉選單、施工日期區間、搜尋與重設按鈕。
- 資料表格：表頭 `bg-slate-800/60 text-slate-400 text-xs`，資料列 `hover:bg-slate-800/40 border-b border-slate-800/60`，金額靠右對齊 `text-right font-mono tabular-nums`。

### 4. 【逾期未收單據】頁面 (Overdue Receivables)
- 頂部雙 KPI 看板：
  - 逾期總金額卡片：`bg-rose-950/20 border border-rose-900/40 text-rose-400`
  - 逾期總筆數卡片：`bg-amber-950/20 border border-amber-900/40 text-amber-400`
- 批次操作列：勾選單據後顯示【批次標記已收費】。
- 逾期表格：醒目標示 `逾期 X 天`（玫瑰紅徽章），列尾快速按鈕【標記為已收費】（翡翠綠按鈕）。

### 5. 【系統設定】頁面 (Settings)
- 卡片式分組：收費期限方案管理表格、自動監聽目錄設定、Ollama 視覺模型參數與測試連線、資料保留週期與過期清理。

---

## 五、 元件代碼速查庫 (Tailwind CSS Snippets)

### 按鈕 (Buttons)
```html
<!-- 主要按鈕 -->
<button class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 active:scale-98 text-white text-sm font-semibold rounded-lg shadow-sm shadow-indigo-600/30 transition-all cursor-pointer">
  確認儲存
</button>

<!-- 次要按鈕 -->
<button class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 active:scale-98 text-slate-200 text-sm font-medium rounded-lg border border-slate-700 transition-all cursor-pointer">
  選擇檔案
</button>

<!-- 收費成功按鈕 -->
<button class="inline-flex items-center justify-center gap-1.5 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-all cursor-pointer">
  標記為已收費
</button>

<!-- 危險刪除按鈕 -->
<button class="inline-flex items-center justify-center gap-1.5 px-3 py-1.5 bg-rose-600/10 hover:bg-rose-600 text-rose-400 hover:text-white border border-rose-500/20 text-xs font-semibold rounded-lg transition-all cursor-pointer">
  刪除單據
</button>
```

### 表單控制元件 (Form Controls)
```html
<!-- 輸入框 -->
<input type="text" class="w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none transition-all font-mono" />

<!-- 下拉選單 -->
<select class="w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none transition-all cursor-pointer">
  <option>選項</option>
</select>
```

### 狀態徽章 (Status Badges)
```html
<!-- 未收費 -->
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/30">● 未收費</span>

<!-- 已收費 -->
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">✓ 已收費</span>

<!-- 逾期警告 -->
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-bold bg-rose-500/10 text-rose-400 border border-rose-500/30">⚠ 逾期 15 天</span>
```

---

## 六、 實作前核對清單

- [ ] 視窗底色使用 `bg-slate-950`，卡片面板使用 `bg-slate-900 border border-slate-800`。
- [ ] 主要操作按鈕統一使用 `bg-indigo-600`。
- [ ] 所有數字、金額、日期、工單號套用 `font-mono tabular-nums`。
- [ ] 狀態色彩嚴格對應：未收費（Amber 琥珀）、已收費（Emerald 翡翠）、逾期未收（Rose 玫瑰紅）。
- [ ] 左側導覽列寬度固定 `w-60`（240px），並具備提醒氣泡紅點。
- [ ] 雙屏審核器左右比例維持 `45% : 55%`。
- [ ] UI 文字一律為 100% 繁體中文，無英文殘留。
