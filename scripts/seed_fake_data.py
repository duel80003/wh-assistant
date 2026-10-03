import sqlite3
import os
import random
from datetime import datetime, timedelta
import base64

DB_PATH = os.path.expanduser("~/Library/Application Support/whassistant/whassistant.db")
IMAGES_DIR = os.path.expanduser("~/Library/Application Support/whassistant/receipts/images")

os.makedirs(os.path.dirname(DB_PATH), exist_ok=True)
os.makedirs(IMAGES_DIR, exist_ok=True)

# Create a small valid sample PNG image for visual preview
SAMPLE_PNG_B64 = (
    "iVBORw0KGgoAAAANSUhEUgAAAMgAAADICAYAAACtWK6eAAAABmJLR0QA/wD/AP+gvaeTAAAACXBIWXMAA"
    "AsTAAALEwEAmpwYAAAAB3RJTUUH6AoDAQkS6j2UAAAAK0lEQVR42u3BAQ0AAADCoPdPbQ43oAAAAAAAAAA"
    "AAAAAAAAAAAAAAAAAAAAAAAAAPBuXpAAB1F1F8AAAAABJRU5ErkJggg=="
)
sample_image_filename = "sample_work_order.png"
sample_image_path = os.path.join(IMAGES_DIR, sample_image_filename)
with open(sample_image_path, "wb") as f:
    f.write(base64.b64decode(SAMPLE_PNG_B64))

conn = sqlite3.connect(DB_PATH)
cursor = conn.cursor()

# Ensure table exists (in case user hasn't run app yet, though schema should exist)
cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='receipts';")
if not cursor.fetchone():
    print("Database table receipts not found. Please launch the app once or run schema migration.")
    exit(1)

# Clean up existing test fake data if needed or check existing max no
cursor.execute("SELECT COALESCE(MAX(no), 10000) FROM receipts;")
max_existing_no = cursor.fetchone()[0]
start_no = max(10001, max_existing_no + 1)

maintainers = [
    "陳大明", "林志偉", "黃建華", "張家豪", "李宗翰",
    "吳冠宇", "蔡承恩", "王俊傑", "劉柏廷", "周子翔",
    "郭廷宇", "許家銘", "鄭文凱", "謝宗祐", "潘建宏"
]

payment_term_choices = [
    (1, 0),   # 即期付款 (0天)
    (2, 30),  # 月結 30 天
    (3, 60),  # 雙月結 60 天
    (4, 90),  # 季結 90 天
]

amounts = [
    850.0, 1200.0, 1500.0, 2400.0, 3200.0, 4500.0, 5600.0,
    6800.0, 8500.0, 9600.0, 12500.0, 15000.0, 18500.0, 22000.0,
    28500.0, 35000.0, 42000.0, 58000.0, 66000.0, 85000.0, 128000.0
]

base_date = datetime(2026, 10, 3) # Current mock baseline date
records = []

# Generate 125 records:
# - 115 'confirmed' (so HistoryView has 12 pages of pagination)
#   - Among confirmed: ~25 are overdue unpaid (due_date < 2026-10-03, payment_status = 'unpaid')
#   - ~60 are paid (payment_status = 'paid')
#   - ~30 are normal upcoming unpaid (due_date >= 2026-10-03, payment_status = 'unpaid')
# - 7 'unconfirmed' (for ReviewView pending list)
# - 3 'failed' (for ReviewView failed items)

for i in range(125):
    current_no = start_no + i
    matainer = random.choice(maintainers)
    amount = random.choice(amounts) + random.randint(0, 99)
    term_id, duration_days = random.choice(payment_term_choices)
    
    # 7 unconfirmed records
    if i < 7:
        status = "unconfirmed"
        work_days_ago = random.randint(1, 10)
        work_dt = base_date - timedelta(days=work_days_ago)
        work_date = work_dt.strftime("%Y-%m-%d")
        due_date = (work_dt + timedelta(days=duration_days)).strftime("%Y-%m-%d")
        payment_status = "unpaid"
        paid_at = None
        error_msg = None
        img_name = sample_image_filename
    # 3 failed records
    elif i < 10:
        status = "failed"
        work_days_ago = random.randint(1, 5)
        work_dt = base_date - timedelta(days=work_days_ago)
        work_date = work_dt.strftime("%Y-%m-%d")
        due_date = (work_dt + timedelta(days=duration_days)).strftime("%Y-%m-%d")
        payment_status = "unpaid"
        paid_at = None
        error_msg = "Ollama 影像辨識失敗：圖片過於模糊無法解析欄位資訊"
        img_name = sample_image_filename
    # 115 confirmed records
    else:
        status = "confirmed"
        # Overdue unpaid (25 records)
        if 10 <= i < 35:
            # Set work date between 60 to 180 days ago
            days_ago = random.randint(60, 200)
            work_dt = base_date - timedelta(days=days_ago)
            work_date = work_dt.strftime("%Y-%m-%d")
            # Ensure due_date is in the past (< base_date)
            due_dt = work_dt + timedelta(days=duration_days)
            if due_dt >= base_date:
                due_dt = base_date - timedelta(days=random.randint(5, 60))
            due_date = due_dt.strftime("%Y-%m-%d")
            payment_status = "unpaid"
            paid_at = None
            error_msg = None
        # Paid records (60 records)
        elif 35 <= i < 95:
            days_ago = random.randint(10, 300)
            work_dt = base_date - timedelta(days=days_ago)
            work_date = work_dt.strftime("%Y-%m-%d")
            due_date = (work_dt + timedelta(days=duration_days)).strftime("%Y-%m-%d")
            payment_status = "paid"
            paid_dt = work_dt + timedelta(days=random.randint(3, max(4, duration_days)))
            paid_at = paid_dt.strftime("%Y-%m-%d %H:%M:%S")
            error_msg = None
        # Upcoming unpaid (30 records)
        else:
            days_ago = random.randint(1, 20)
            work_dt = base_date - timedelta(days=days_ago)
            work_date = work_dt.strftime("%Y-%m-%d")
            # Ensure due_date is in the future (>= base_date)
            due_dt = base_date + timedelta(days=random.randint(10, 60))
            due_date = due_dt.strftime("%Y-%m-%d")
            payment_status = "unpaid"
            paid_at = None
            error_msg = None
        
        img_name = sample_image_filename if random.random() > 0.3 else f"receipt_{current_no}.jpg"

    now_iso = datetime.now().isoformat()
    created_at = (base_date - timedelta(days=random.randint(0, 30))).strftime("%Y-%m-%d %H:%M:%S")
    updated_at = created_at

    records.append((
        current_no,
        matainer,
        work_date,
        due_date,
        amount,
        "TWD",
        img_name,
        status,
        payment_status,
        term_id,
        paid_at,
        error_msg,
        created_at,
        updated_at
    ))

cursor.executemany(
    """
    INSERT INTO receipts (
        no, matainer, work_date, due_date, total_amount, currency, image_path,
        status, payment_status, payment_term_id, paid_at, error_message, created_at, updated_at
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """,
    records
)

conn.commit()

# Print statistics
cursor.execute("SELECT count(*) FROM receipts;")
total_count = cursor.fetchone()[0]

cursor.execute("SELECT count(*) FROM receipts WHERE status = 'confirmed';")
confirmed_count = cursor.fetchone()[0]

cursor.execute("SELECT count(*) FROM receipts WHERE status IN ('unconfirmed', 'failed', 'processing');")
pending_count = cursor.fetchone()[0]

cursor.execute("SELECT count(*), SUM(total_amount) FROM receipts WHERE payment_status = 'unpaid' AND status = 'confirmed' AND due_date < '2026-10-03';")
overdue_row = cursor.fetchone()

conn.close()

print(f"Successfully inserted {len(records)} fake receipt records!")
print(f"Total receipts in DB: {total_count}")
print(f"Confirmed receipts (HistoryView, 10/page => {confirmed_count // 10 + 1} pages): {confirmed_count}")
print(f"Pending/Unconfirmed (ReviewView badge): {pending_count}")
print(f"Overdue unpaid (OverdueView): {overdue_row[0]} records, Total amount: NT$ {overdue_row[1]:.2f}")
