# WHassistant 單據管理系統 - UI/UX 設計規範與視覺體系指引 (UI Design System)

> **前言**：本文件定義全系統統一之色彩、字體、佈局、狀態反饋與核心元件樣式標準。**後續所有前端（Dioxus 0.7 + Tailwind CSS）介面實作必須嚴格遵守本規範**，確保整體視覺語言一致、專業、具備現代感與高易用性。

---

## 一、 設計風格與原則

1. **專業工控與高密度資訊流**：介面以清楚傳遞數據為核心，兼顧工單影像核對效率與財務催收提醒。
2. **現代深色為主、高對比易讀（Modern Slate Dark）**：
   - 預設採用精緻的 **Slate 灰階深色主題**（低眩光、護眼、能突顯單據圖片與彩色狀態徽章）。
   - 避免純黑（`#000000`），全面採用 Slate 系列（`slate-950`、`slate-900`、`slate-800`）營造細緻層次。
3. **語意化色彩一致性**：收費狀態、逾期警示與操作反饋在所有頁面保持絕對一致的顏色對應。
4. **數字排版對齊**：金額、工單號、日期及逾期天數一律採用等寬數字字型（`tabular-nums font-mono`），確保直欄對齊。

---

## 二、 色彩體系 (Color Palette)

全系統基於 Tailwind CSS 規範色彩調色盤，主要分為五大維度：

```
+-------------------------------------------------------------------------+
| 主色彩 (Primary)      : 靛藍 Indigo-600 (#4f46e5) -> 品牌、主操作、導覽列選取   |
| 次要色彩 (Secondary)  : 岩灰 Slate-700 (#334155)  -> 次級按鈕、邊框、圖表軸線   |
| 背景基底 (Surface)    : 深岩 Slate-950 (#020617)  -> 視窗底色、輸入框背景       |
| 卡片層級 (Elevation)  : 灰藍 Slate-900 (#0f172a)  -> 模組卡片、對話框、側邊欄   |
| 邊框線條 (Border)     : 邊框 Slate-800 (#1e293b)  -> 分隔線、卡片輪廓           |
+-------------------------------------------------------------------------+
```

### 1. 品牌與介面基底色系

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

### 2. 字級與層級規範 (Scale & Hierarchy)

| 層級名稱 | 尺寸 (Tailwind) | 粗細 (Weight) | 色彩 (Text Color) | 應用場合 |
| :--- | :--- | :--- | :--- | :--- |
| **頁面主標題 (H1)** | `text-2xl` (24px) | `font-bold` (700) | `text-slate-100` | 頁面頂部功能大標題 |
| **看板重點數字 (Metric)**| `text-3xl` (30px) | `font-extrabold` (800)| `text-rose-400` / `text-emerald-400` | 逾期總金額、待收筆數 |
| **區塊標題 (H2)** | `text-lg` (18px) | `font-semibold` (600) | `text-slate-200` | 卡片標題、分屏區塊標題 |
| **主要內文 (Body)** | `text-sm` (14px) | `font-normal` (400) | `text-slate-300` | 表格內容、表單欄位文字 |
| **次要說明 (Caption)** | `text-xs` (12px) | `font-normal` (400) | `text-slate-400` | 欄位提示字、建立時間、輔助標籤 |
| **微小註記 (Micro)** | `text-[10px]` (10px) | `font-medium` (500) | `text-slate-500` | 縮圖角標、圖示微小徽章 |

---

## 四、 核心版面結構與導覽佈局 (App Shell Layout)

整體桌面視窗採「**左側固定側邊欄 + 右側主內容流**」之經典高效架構（支援視窗最小寬度 1100px）：

```
+------------------+-------------------------------------------------------------+
|  WHassistant     | 頂部導覽列 (Breadcrumb + 狀態摘要 + 視窗輔助工具)            |
|  [Logo & 系統名] |                                                             |
+------------------+-------------------------------------------------------------+
|  [待審單據] (5)  |                                                             |
|  [歷史單據]      |                   主要工作區內容容器                         |
|  [逾期未收] (2)  |                (Main Content Container)                     |
|  [系統設定]      |                  (自動滾動 overflow-y-auto)                  |
|                  |                                                             |
|                  |                                                             |
|------------------|                                                             |
|  [Ollama 連線態] |                                                             |
|  [v0.1.0 繁體]   |                                                             |
+------------------+-------------------------------------------------------------+
```

### 1. 左側導覽列 (Sidebar) - 固定寬度 240px
- **容器樣式**：`w-60 bg-slate-900 border-r border-slate-800 flex flex-col justify-between p-4 shrink-0`
- **導覽按鈕樣式**：
  - **一般未選中**：`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors`
  - **選中狀態 (Active)**：`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-white bg-indigo-600/90 shadow-sm shadow-indigo-500/20`
- **紅點徽章 (Badge)**：
  - 當有待確認單據或逾期未收單據時，按鈕右側顯示計數圓點：`ml-auto px-2 py-0.5 rounded-full text-xs font-bold bg-rose-500 text-white animate-pulse`

### 2. 主內容區域 (Main Content Area)
- **容器樣式**：`flex-1 h-screen overflow-y-auto bg-slate-950 p-6 flex flex-col gap-6`

---

## 五、 四大核心功能頁面版面設計標準

### 1. 【待審單據】頁面版面 (Review Queue / Split-View)
- **頂部拖曳上傳橫幅 (Drop Zone Banner)**：
  - 高度：`h-28`
  - 樣式：`border-2 border-dashed border-slate-700 hover:border-indigo-500 rounded-xl bg-slate-900/50 hover:bg-indigo-950/20 flex flex-col items-center justify-center cursor-pointer transition-all`
  - 提示文字：「拖曳工單圖片至此處，或點擊下方按鈕上傳」＋ 【選擇圖片檔案】次級按鈕。
- **核心雙屏檢視器 (Split Screen 45% : 55%)**：
  - **左側圖片檢視區 (45% 寬度)**：
    - 樣式：`bg-slate-900 border border-slate-800 rounded-xl p-4 flex flex-col items-center justify-center relative overflow-hidden min-h-[500px]`
    - 工具列：浮動置頂工具列（放大、縮小、旋轉、原始大小），樣式：`absolute top-3 right-3 bg-slate-800/80 backdrop-blur rounded-lg p-1.5 flex gap-1 border border-slate-700`
  - **右側資料表單區 (55% 寬度)**：
    - 樣式：`bg-slate-900 border border-slate-800 rounded-xl p-6 flex flex-col justify-between`
    - 表單欄位：採 2 欄網格排版（`grid grid-cols-2 gap-4`）。
    - 底部操作列：`flex items-center justify-between pt-6 border-t border-slate-800`
      - 左側：【刪除單據】（危險色）、【重新辨識】
      - 右側：【上一筆】、【下一筆】、主要按鈕【確認儲存】

### 2. 【歷史單據】頁面版面 (History Archive)
- **頂部搜尋篩選列 (Filter Bar)**：
  - 樣式：`bg-slate-900 border border-slate-800 rounded-xl p-4 flex flex-wrap items-center gap-3`
  - 元件：搜尋框（`w-64`）、收費狀態下拉選單（全部／未收費／已收費）、施工日期區間選擇器、【搜尋】、【重設】按鈕。
- **資料分頁表格 (Data Table)**：
  - 樣式：`w-full text-left border-collapse`
  - 表頭：`bg-slate-800/60 text-slate-400 text-xs font-semibold py-3 px-4 uppercase tracking-wider border-b border-slate-800`
  - 內容列：`hover:bg-slate-800/40 border-b border-slate-800/60 transition-colors py-3 px-4 text-sm text-slate-300`
  - 金額欄位：`text-right font-mono tabular-nums font-semibold text-slate-100`
- **分頁控制列 (Pagination)**：
  - 樣式：`flex items-center justify-between pt-4 text-xs text-slate-400`

### 3. 【逾期未收單據】頁面版面 (Overdue Receivables)
- **頂部雙核心 KPI 警示看板 (2-Card Alert Banner)**：
  - 網格：`grid grid-cols-2 gap-6`
  - 卡片 A（逾期總金額）：`bg-rose-950/20 border border-rose-900/40 rounded-xl p-5 flex flex-col gap-1`
    - 標籤：「逾期未收總金額」`text-xs font-semibold text-rose-400`
    - 數值：`text-3xl font-extrabold text-rose-400 font-mono tracking-tight`
  - 卡片 B（逾期總筆數）：`bg-amber-950/20 border border-amber-900/40 rounded-xl p-5 flex flex-col gap-1`
    - 標籤：「逾期單據總筆數」`text-xs font-semibold text-amber-400`
    - 數值：`text-3xl font-extrabold text-amber-400 font-mono tracking-tight`
- **批次處理列 (Batch Action Bar)**：
  - 當勾選 1 筆以上時浮現：`bg-slate-900 border border-indigo-500/50 rounded-lg p-3 flex items-center justify-between`
  - 【批次標記已收費】綠色主要按鈕。
- **逾期明細表 (Overdue Table)**：
  - 醒目顯示「已逾期天數」：紅色高對比徽章（例如：`逾期 15 天`）。
  - 單項列尾快速動作：按鈕【標記為已收費】（直接綠色實心按鈕，點擊即完成銷帳）。

### 4. 【系統設定】頁面版面 (Settings)
- 採單欄或雙欄直式卡片群組（`flex flex-col gap-6 max-w-4xl`）：
  - **卡片 1：收費期限方案管理（Payment Terms）**：表格展示現行方案、新增方案對話框。
  - **卡片 2：資料自動監聽設定**：資料夾路徑選擇器、開關切換器。
  - **卡片 3：Ollama 視覺模型連接**：Base URL、模型名稱、連線測試指示燈。
  - **卡片 4：資料保留週期與清理**：下拉天數選擇、危險操作區【立即清理已收費過期單據】。

---

## 六、 常用元件標準樣式庫 (Component Reference Tokens)

各畫面實作元件時，請直接使用下列 Tailwind CSS 類別組合：

### 1. 按鈕組 (Buttons)

```html
<!-- 1. 主要操作按鈕 (Primary Button) -->
<button class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 active:scale-98 text-white text-sm font-semibold rounded-lg shadow-sm shadow-indigo-600/30 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed">
  確認儲存
</button>

<!-- 2. 次要/中性按鈕 (Secondary Button) -->
<button class="inline-flex items-center justify-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 active:scale-98 text-slate-200 text-sm font-medium rounded-lg border border-slate-700 transition-all cursor-pointer">
  選擇檔案
</button>

<!-- 3. 收費完成/成功按鈕 (Success Action Button) -->
<button class="inline-flex items-center justify-center gap-1.5 px-3 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-all cursor-pointer">
  標記為已收費
</button>

<!-- 4. 危險操作按鈕 (Danger Button) -->
<button class="inline-flex items-center justify-center gap-1.5 px-3 py-1.5 bg-rose-600/10 hover:bg-rose-600 text-rose-400 hover:text-white border border-rose-500/20 text-xs font-semibold rounded-lg transition-all cursor-pointer">
  刪除單據
</button>

<!-- 5. 幽靈/圖示按鈕 (Ghost Icon Button) -->
<button class="p-2 text-slate-400 hover:text-slate-100 hover:bg-slate-800 rounded-lg transition-colors cursor-pointer">
  <!-- SVG Icon -->
</button>
```

### 2. 表單輸入框 (Form Controls)

```html
<!-- 文字輸入框 (Input) -->
<div class="flex flex-col gap-1.5">
  <label class="text-xs font-medium text-slate-300">工單號碼</label>
  <input type="text" placeholder="例如：NO.12345678" 
    class="w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none transition-all font-mono" />
</div>

<!-- 下拉選單 (Select) -->
<div class="flex flex-col gap-1.5">
  <label class="text-xs font-medium text-slate-300">收費期限方案</label>
  <select class="w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none transition-all cursor-pointer">
    <option value="1">月結 30 天 (預設)</option>
    <option value="2">雙月結 60 天</option>
    <option value="0">即期付款</option>
  </select>
</div>
```

### 3. 業務狀態徽章 (Status Badges)

```html
<!-- 未收費徽章 (Unpaid) -->
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/30">
  ● 未收費
</span>

<!-- 已收費徽章 (Paid) -->
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/30">
  ✓ 已收費
</span>

<!-- 逾期警告徽章 (Overdue Days) -->
<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-bold bg-rose-500/10 text-rose-400 border border-rose-500/30">
  ⚠ 逾期 15 天
</span>

<!-- 處理中動態徽章 (Processing) -->
<span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-sky-500/10 text-sky-400 border border-sky-500/30 animate-pulse">
  <span class="w-1.5 h-1.5 rounded-full bg-sky-400 animate-ping"></span> 辨識中
</span>
```

### 4. 模組卡片容器 (Card Container)

```html
<div class="bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4">
  <div class="flex items-center justify-between border-b border-slate-800 pb-3">
    <h3 class="text-sm font-semibold text-slate-200">卡片標題</h3>
  </div>
  <div>
    <!-- 卡片內容 -->
  </div>
</div>
```

### 5. 對話框遮罩與面板 (Modal Dialog)

```html
<!-- 遮罩背景 -->
<div class="fixed inset-0 bg-black/70 backdrop-blur-xs z-50 flex items-center justify-center p-4">
  <!-- 對話框容器 -->
  <div class="bg-slate-900 border border-slate-800 rounded-2xl max-w-lg w-full p-6 shadow-2xl flex flex-col gap-5">
    <div class="flex items-center justify-between">
      <h3 class="text-base font-bold text-slate-100">刪除單據確認</h3>
      <button class="text-slate-400 hover:text-slate-200">✕</button>
    </div>
    <p class="text-sm text-slate-300 leading-relaxed">
      確定要刪除此工單嗎？關聯的圖片檔案將一併清除，此動作無法復原。
    </p>
    <div class="flex items-center justify-end gap-3 pt-2">
      <button class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium rounded-lg">取消</button>
      <button class="px-4 py-2 bg-rose-600 hover:bg-rose-500 text-white text-sm font-semibold rounded-lg">確認刪除</button>
    </div>
  </div>
</div>
```

---

## 七、 實作前核對清單 (Implementation Checklist)

工程師與 Agent 在撰寫 Dioxus 介面程式碼時，請逐項檢查：
- [ ] 視窗底色是否使用 `bg-slate-950`，卡片是否使用 `bg-slate-900 border border-slate-800`？
- [ ] 主按鈕顏色是否統一為 `bg-indigo-600`，懸停為 `bg-indigo-500`？
- [ ] 所有數字、金額、日期是否套用 `font-mono tabular-nums`？
- [ ] 「未收費」標籤是否使用琥珀金（Amber），「已收費」標籤是否使用翡翠綠（Emerald），「逾期未收」是否使用玫瑰紅（Rose）？
- [ ] 左側導覽列寬度是否維持固定 `w-60`（240px），並帶有選中與計數紅點樣式？
- [ ] 雙屏審核器的左右比例是否維持 `45% : 55%`，且原圖具備縮放平移容器？
- [ ] 介面所有按鈕、欄位、標籤是否 100% 全部使用繁體中文，無任何英文夾雜？
