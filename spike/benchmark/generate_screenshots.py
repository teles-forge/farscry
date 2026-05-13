
"""
farscry benchmark - screenshot generator
Creates 20 realistic dev-tool screenshots with known ground truth.
Each screenshot has a pre-defined question and exact correct answer
so evaluation is objective (not subjective).

Design: mix of cases where raw vision wins, where farscry wins,
and where they tie - no thumb on the scale.
"""

import json
import os
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).parent / "screenshots"
OUT.mkdir(exist_ok=True)

try:
    MONO_LG  = ImageFont.truetype("/System/Library/Fonts/Monaco.ttf", 14)
    MONO_MD  = ImageFont.truetype("/System/Library/Fonts/Monaco.ttf", 12)
    MONO_SM  = ImageFont.truetype("/System/Library/Fonts/Monaco.ttf", 11)
    UI_LG    = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 14)
    UI_MD    = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 12)
    UI_SM    = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 11)
    UI_BOLD  = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 13)
except Exception:
    MONO_LG = MONO_MD = MONO_SM = UI_LG = UI_MD = UI_SM = UI_BOLD = ImageFont.load_default()

DARK_BG   = "#1e1e1e"
DARK_BG2  = "#252526"
DARK_BG3  = "#2d2d30"
PANEL_BG  = "#333333"
BORDER    = "#474747"
WHITE     = "#cccccc"
DIM       = "#858585"
RED       = "#f44747"
RED2      = "#f48771"
YELLOW    = "#ffcc00"
GREEN     = "#4ec9b0"
BLUE      = "#569cd6"
CYAN      = "#9cdcfe"
ORANGE    = "#ce9178"
PURPLE    = "#c586c0"
LIGHT_BG  = "#f5f5f5"
LIGHT_TXT = "#1e1e1e"
LIGHT_DIM = "#6e6e6e"
LIGHT_ERR = "#d32f2f"
LIGHT_BRD = "#ddd"
BTN_BLUE  = "#0078d4"
BTN_GRAY  = "#737373"
BTN_RED   = "#d32f2f"
BTN_GREEN = "#2e7d32"

def new(w=800, h=500, bg=DARK_BG):
    img = Image.new("RGB", (w, h), bg)
    return img, ImageDraw.Draw(img)

def rect(d, x, y, w, h, fill=None, outline=None, r=0):
    if r:
        d.rounded_rectangle([x, y, x+w, y+h], radius=r, fill=fill, outline=outline)
    else:
        d.rectangle([x, y, x+w, y+h], fill=fill, outline=outline)

def text(d, x, y, s, font=None, fill=WHITE):
    d.text((x, y), s, font=font or MONO_MD, fill=fill)

def button(d, x, y, label, w=None, color=BTN_BLUE, fg="#ffffff", disabled=False, font=None):
    bw = w or (len(label) * 8 + 20)
    bh = 28
    bg = "#555555" if disabled else color
    rect(d, x, y, bw, bh, fill=bg, r=4)
    text(d, x + (bw - len(label)*7)//2, y+7, label, font=font or UI_MD, fill=fg if not disabled else "#999999")
    return x, y, bw, bh

def tab_bar(d, x, y, tabs, active=0, bg=DARK_BG2):
    cx = x
    for i, t in enumerate(tabs):
        tw = len(t) * 7 + 20
        color = DARK_BG if i == active else DARK_BG3
        rect(d, cx, y, tw, 32, fill=color)
        text(d, cx+10, y+9, t, font=UI_MD, fill=WHITE if i==active else DIM)
        cx += tw

def line_numbers(d, x, y, start, count, font=None):
    for i in range(count):
        text(d, x, y + i*18, str(start+i), font=font or MONO_SM, fill=DIM)

GROUND_TRUTH = {}

def make_01():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 320, 10, "bash - 80x24", font=UI_MD, fill=WHITE)
    lines = [
        ("$ python3 app.py",          "#4ec9b0", 14),
        ("Traceback (most recent call last):", "#cccccc", 14),
        ('  File "app.py", line 23, in <module>', "#cccccc", 14),
        ("    result = processor.process(data)", "#cccccc", 14),
        ('  File "processor.py", line 47, in process', "#cccccc", 14),
        ("    return self.pipeline.run(data)",   "#cccccc", 14),
        ("AttributeError: 'NoneType' object has no attribute 'run'", "#f44747", 14),
        ("",                           "#cccccc", 14),
        ("The pipeline was not initialized. Call .setup() first.", "#ffcc00", 14),
        ("",                           "#cccccc", 14),
        ("$ _",                        "#4ec9b0", 14),
    ]
    y = 55
    for line, color, _ in lines:
        text(d, 20, y, line, font=MONO_MD, fill=color)
        y += 22
    img.save(OUT / "01.png")
    GROUND_TRUTH["01.png"] = {
        "question": "What is the error and which file + line caused it?",
        "correct_element": "processor.py line 47",
        "correct_action": "NoneType has no attribute 'run' in processor.py:47 - pipeline not initialized",
        "category": "terminal",
        "advantage": "tie",
    }

def make_02():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 280, 10, "cargo build - farscry", font=UI_MD, fill=WHITE)
    lines = [
        ("   Compiling farscry-core v0.1.0",       "#858585"),
        ("error[E0382]: borrow of moved value: `output`", "#f44747"),
        (" --> src/pipeline.rs:89:18",              "#4ec9b0"),
        ("  |",                                     "#858585"),
        ("87 |     let output = self.ocr.extract(&img)?;", "#cccccc"),
        ("   |         ------  value moved here",  "#858585"),
        ("88 |     log::debug!(\"{:?}\", output);",  "#cccccc"),
        ("   |                                ^^^^^^ value used here after move", "#f44747"),
        ("89 |     self.formatter.format(output)",  "#cccccc"),
        ("   |                          ^^^^^^",    "#f44747"),
        ("",                                        "#cccccc"),
        ("For more information: error --explain E0382", "#858585"),
        ("error: could not compile `farscry-core`", "#f44747"),
    ]
    y = 50
    for line, color in lines:
        text(d, 20, y, line, font=MONO_SM, fill=color)
        y += 20
    img.save(OUT / "02.png")
    GROUND_TRUTH["02.png"] = {
        "question": "What is the compile error and which file and line?",
        "correct_element": "src/pipeline.rs line 89",
        "correct_action": "borrow of moved value 'output' at pipeline.rs:89 - E0382",
        "category": "terminal",
        "advantage": "tie",
    }

def make_03():
    img, d = new(800, 500, LIGHT_BG)
    rect(d, 0, 0, 800, 56, fill="#0078d4")
    text(d, 24, 18, "Payment Settings", font=UI_BOLD, fill="#ffffff")
    rect(d, 40, 72, 720, 380, fill="#ffffff", outline=LIGHT_BRD)
    text(d, 64, 100, "Card Number", font=UI_MD, fill=LIGHT_DIM)
    rect(d, 64, 120, 672, 38, fill="#fff", outline="#cc0000")
    text(d, 72, 130, "4532 •••• •••• ••••", font=MONO_MD, fill=LIGHT_TXT)
    text(d, 64, 162, "Card number is invalid", font=UI_SM, fill=LIGHT_ERR)
    text(d, 64, 185, "Expiry", font=UI_MD, fill=LIGHT_DIM)
    rect(d, 64, 205, 200, 38, fill="#fff", outline=LIGHT_BRD)
    text(d, 72, 215, "12 / 26", font=MONO_MD, fill=LIGHT_TXT)
    text(d, 280, 185, "CVV", font=UI_MD, fill=LIGHT_DIM)
    rect(d, 280, 205, 120, 38, fill="#fff", outline=LIGHT_BRD)
    text(d, 288, 215, "•••", font=MONO_MD, fill=LIGHT_TXT)
    text(d, 64, 268, "Billing Address", font=UI_MD, fill=LIGHT_DIM)
    rect(d, 64, 288, 672, 38, fill="#fff", outline=LIGHT_BRD)
    text(d, 72, 298, "123 Main Street, San Francisco, CA", font=MONO_MD, fill=LIGHT_TXT)
    bx, by = 560, 370
    rect(d, bx, by, 160, 36, fill=BTN_GRAY, r=4)
    text(d, bx+30, by+9, "Save Payment", font=UI_MD, fill="#cccccc")
    rect(d, 420, 370, 120, 36, fill="#ffffff", outline=LIGHT_BRD, r=4)
    text(d, 447, 379, "Cancel", font=UI_MD, fill=LIGHT_TXT)
    img.save(OUT / "03.png")
    GROUND_TRUTH["03.png"] = {
        "question": "Why is the Save Payment button disabled? What field has an error?",
        "correct_element": "Card Number field at top of form",
        "correct_action": "Card number is invalid - fix the card number field to enable Save Payment button",
        "category": "web_form",
        "advantage": "farscry",
    }

def make_04():
    img, d = new(800, 480, DARK_BG)
    rect(d, 0, 0, 800, 36, fill=DARK_BG3)
    text(d, 20, 10, "PROBLEMS   TERMINAL   OUTPUT   DEBUG CONSOLE", font=UI_MD, fill=DIM)
    text(d, 20, 10, "PROBLEMS", font=UI_BOLD, fill=WHITE)
    rect(d, 680, 8, 100, 22, fill="#5a1d1d", r=3)
    text(d, 690, 11, "● 3 errors  ◆ 1 warning", font=UI_SM, fill=RED)
    errors = [
        ("●", "src/components/ProductList.tsx", "47", "Type 'undefined' is not assignable to type 'Product[]'"),
        ("●", "src/components/ProductList.tsx", "52", "Property 'map' does not exist on type 'undefined'"),
        ("●", "src/pages/Checkout.tsx",         "103", "Cannot find name 'CartItem'. Did you mean 'CartItems'?"),
        ("◆", "src/utils/api.ts",               "23",  "Variable 'response' is used before assignment"),
    ]
    y = 55
    for i, (icon, fname, lineno, msg) in enumerate(errors):
        bg = "#2a1515" if icon == "●" else "#2a2a10"
        rect(d, 0, y, 800, 32, fill=bg)
        fill = RED if icon == "●" else YELLOW
        text(d, 16, y+8, icon, font=UI_MD, fill=fill)
        text(d, 38, y+8, msg, font=MONO_SM, fill=WHITE)
        text(d, 38, y+20, f"{fname}:{lineno}", font=MONO_SM, fill=DIM)
        y += 34
    img.save(OUT / "04.png")
    GROUND_TRUTH["04.png"] = {
        "question": "Which file has the most errors and what is the first error on line 47?",
        "correct_element": "src/components/ProductList.tsx - 2 errors",
        "correct_action": "ProductList.tsx has 2 errors; line 47: Type 'undefined' is not assignable to type 'Product[]'",
        "category": "vscode",
        "advantage": "farscry",
    }

def make_05():
    img, d = new(800, 480, LIGHT_BG)
    rect(d, 0, 0, 800, 52, fill="#24292f")
    text(d, 24, 16, "github.com / teles-forge / farscry / pull / 42", font=UI_MD, fill="#8b949e")
    text(d, 24, 80, "Fix: initialize pipeline before calling .run()", font=UI_LG, fill=LIGHT_TXT)
    rect(d, 24, 110, 74, 22, fill="#0075ca", r=11)
    text(d, 33, 113, "enhancement", font=UI_SM, fill="#ffffff")
    rect(d, 24, 148, 752, 110, fill="#ffffff", outline="#d0d7de")
    rect(d, 24, 148, 752, 38, fill="#fff8c5", outline="#d0d7de")
    text(d, 48, 160, "● Review required", font=UI_BOLD, fill="#9a6700")
    text(d, 48, 186, "At least 1 approving review is required by a code owner before merging.", font=UI_SM, fill=LIGHT_DIM)
    text(d, 48, 206, "Required reviewer: @arch-bot has not reviewed yet.", font=UI_SM, fill=LIGHT_DIM)
    text(d, 48, 226, "All checks passed  OK", font=UI_SM, fill=BTN_GREEN)
    bx, by = 24, 272
    rect(d, bx, by, 200, 36, fill=BTN_GRAY, r=6)
    text(d, bx+30, by+10, "Merge pull request", font=UI_MD, fill="#aaaaaa")
    text(d, 24, 320, "You can also open this in GitHub.dev  |  View command line instructions", font=UI_SM, fill="#0969da")
    img.save(OUT / "05.png")
    GROUND_TRUTH["05.png"] = {
        "question": "Why is the Merge button disabled and what action is required?",
        "correct_element": "Merge pull request button (disabled)",
        "correct_action": "Review required - @arch-bot must approve before merging",
        "category": "web_form",
        "advantage": "tie",
    }

def make_06():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 300, 10, "npm audit", font=UI_MD, fill=WHITE)
    lines = [
        ("$ npm audit",                         GREEN),
        ("",                                    WHITE),
        ("# npm audit report",                  DIM),
        ("",                                    WHITE),
        ("semver  <7.5.2",                       RED),
        ("Severity: critical",                   RED),
        ("Regular Expression Denial of Service", WHITE),
        ("fix available via `npm audit fix`",    GREEN),
        ("node_modules/semver",                  DIM),
        ("  node_modules/lerna/node_modules/semver", DIM),
        ("",                                    WHITE),
        ("path-to-regexp  <0.1.10",              YELLOW),
        ("Severity: high",                       YELLOW),
        ("ReDoS in path-to-regexp",              WHITE),
        ("",                                    WHITE),
        ("3 vulnerabilities (1 critical, 2 high)", RED),
        ("",                                    WHITE),
        ("Run `npm audit fix` to fix them.",    GREEN),
    ]
    y = 50
    for line, color in lines:
        text(d, 20, y, line, font=MONO_SM, fill=color)
        y += 18
    img.save(OUT / "06.png")
    GROUND_TRUTH["06.png"] = {
        "question": "How many critical vulnerabilities are there and in which package?",
        "correct_element": "semver - 1 critical vulnerability",
        "correct_action": "1 critical vulnerability in semver (<7.5.2) - run npm audit fix",
        "category": "terminal",
        "advantage": "tie",
    }

def make_07():
    img, d = new(800, 500, LIGHT_BG)
    rect(d, 0, 0, 800, 52, fill="#1976d2")
    text(d, 24, 16, "Application Settings", font=UI_BOLD, fill="#ffffff")
    rect(d, 40, 72, 720, 390, fill="#ffffff", outline=LIGHT_BRD)
    fields = [
        ("API Base URL",   "https://api.example.com",    None,    "#1976d2"),
        ("Timeout (ms)",   "30000",                       None,    "#1976d2"),
        ("Max Retries",    "999",                         "Value must be between 1 and 10", "#cc0000"),
        ("Auth Token",     "sk-••••••••••••••••2f9a",    None,    "#1976d2"),
        ("Webhook URL",    "https://hooks.example.com",  None,    "#1976d2"),
    ]
    y = 88
    for label, val, err, border_color in fields:
        text(d, 64, y, label, font=UI_MD, fill=LIGHT_DIM)
        border = "#cc0000" if err else LIGHT_BRD
        rect(d, 64, y+20, 672, 36, fill="#fff", outline=border)
        text(d, 72, y+29, val, font=MONO_MD, fill=LIGHT_TXT)
        if err:
            text(d, 64, y+60, f"  {err}", font=UI_SM, fill=LIGHT_ERR)
            y += 82
        else:
            y += 66
    bx = 560
    rect(d, bx, 430, 160, 36, fill=BTN_GRAY, r=4)
    text(d, bx+30, 439, "Save Settings", font=UI_MD, fill="#cccccc")
    rect(d, 420, 430, 120, 36, fill="#ffffff", outline=LIGHT_BRD, r=4)
    text(d, 447, 439, "Cancel", font=UI_MD, fill=LIGHT_TXT)
    img.save(OUT / "07.png")
    GROUND_TRUTH["07.png"] = {
        "question": "Which field has a validation error and what is the current invalid value?",
        "correct_element": "Max Retries field",
        "correct_action": "Max Retries has value 999 which is invalid (must be 1-10)",
        "category": "config",
        "advantage": "farscry",
    }

def make_08():
    img, d = new(800, 480, "#1f1f1f")
    rect(d, 0, 0, 800, 36, fill="#292929")
    tabs = ["Elements", "Console", "Sources", "Network", "Performance"]
    tab_bar(d, 0, 0, tabs, active=1, bg="#292929")
    rect(d, 0, 36, 800, 1, fill="#404040")
    entries = [
        ("error",   "Access to fetch at 'https://api.internal.corp/v1/users' from origin 'http://localhost:3000' has been blocked by CORS policy: No 'Access-Control-Allow-Origin' header."),
        ("info",    "GET https://api.internal.corp/v1/users  net::ERR_FAILED 0"),
        ("error",   "Uncaught (in promise) TypeError: Failed to fetch"),
        ("warning", "React does not recognize the `isActive` prop on a DOM element."),
        ("log",     "Component mounted: UserDashboard"),
        ("log",     "Fetching user list from API..."),
    ]
    y = 48
    icons = {"error": ("●", RED), "warning": ("▲", YELLOW), "info": ("", BLUE), "log": ("", WHITE)}
    for level, msg in entries:
        bg = "#2a1515" if level == "error" else ("#2a2a10" if level == "warning" else "#1f1f1f")
        rect(d, 0, y, 800, 30, fill=bg)
        icon, color = icons[level]
        if icon:
            text(d, 8, y+8, icon, font=UI_SM, fill=color)
        text(d, 24, y+8, msg[:100], font=MONO_SM, fill=color)
        y += 31
    img.save(OUT / "08.png")
    GROUND_TRUTH["08.png"] = {
        "question": "What URL is being blocked by CORS and from what origin?",
        "correct_element": "https://api.internal.corp/v1/users",
        "correct_action": "CORS blocks api.internal.corp/v1/users from localhost:3000 - server needs Access-Control-Allow-Origin header",
        "category": "devtools",
        "advantage": "tie",
    }

def make_09():
    img, d = new(800, 480, DARK_BG)
    rect(d, 0, 0, 800, 36, fill=DARK_BG3)
    text(d, 24, 10, "ProductList.tsx - farscry-ui", font=UI_MD, fill=DIM)
    text(d, 24, 10, "ProductList.tsx", font=UI_MD, fill=WHITE)
    lines = [
        (20, "import React from 'react';",                         WHITE),
        (21, "import { Product } from '../types';",                WHITE),
        (22, "",                                                   WHITE),
        (23, "interface Props { products: Product[] | undefined; }", RED),
        (24, "",                                                   WHITE),
        (25, "const ProductList: React.FC<Props> = ({ products }) => {", WHITE),
        (26, "  return (",                                         WHITE),
        (27, "    <div className=\"product-grid\">",               ORANGE),
        (28, "      {products.map((p) => (", WHITE),
        (29, "        <div key={p.id}>{p.name}</div>",             WHITE),
        (30, "      ))}",                                          WHITE),
        (31, "    </div>",                                         ORANGE),
        (32, "  );",                                               WHITE),
        (33, "};",                                                 WHITE),
    ]
    y = 48
    for lineno, code, color in lines:
        bg = "#3a1515" if lineno == 23 else DARK_BG
        rect(d, 0, y, 800, 18, fill=bg)
        text(d, 8, y, str(lineno), font=MONO_SM, fill=DIM)
        text(d, 48, y, code, font=MONO_SM, fill=color)
        if lineno == 23:
            text(d, 48, y, "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~", font=MONO_SM, fill=RED)
        y += 18
    rect(d, 0, y+10, 800, 50, fill="#5a1d1d")
    text(d, 16, y+18, "● TypeScript error (2345): Argument of type 'Product[] | undefined' is not assignable", font=MONO_SM, fill="#f48771")
    text(d, 16, y+34, "   to parameter of type 'Product[]'. Type 'undefined' is not assignable to type 'Product[]'.", font=MONO_SM, fill="#f48771")
    img.save(OUT / "09.png")
    GROUND_TRUTH["09.png"] = {
        "question": "What is the TypeScript error on line 23 and how to fix it?",
        "correct_element": "line 23 - interface Props definition",
        "correct_action": "TS2345: undefined not assignable to Product[] - add null check or change type to Product[]",
        "category": "vscode",
        "advantage": "tie",
    }

def make_10():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 260, 10, "docker compose up - farscry-stack", font=UI_MD, fill=WHITE)
    lines = [
        ("$ docker compose up",                          GREEN),
        ("[+] Running 3/3",                             WHITE),
        (" OK Container farscry-db         Started",     GREEN),
        (" OK Container farscry-redis      Started",     GREEN),
        (" ✘ Container farscry-api        Error",       RED),
        ("",                                            WHITE),
        ("Error response from daemon:",                 RED),
        ("  driver failed programming external connectivity", RED),
        ("  on endpoint farscry-api (sha256:d3b4...):  ",     RED),
        ("  Bind for 0.0.0.0:8080 failed: port is already allocated", RED),
        ("",                                            WHITE),
        ("Hint: Check if something is already running on port 8080.", YELLOW),
        ("  lsof -i :8080  or  netstat -an | grep 8080", DIM),
        ("",                                            WHITE),
        ("$ _",                                         GREEN),
    ]
    y = 52
    for line, color in lines:
        text(d, 20, y, line, font=MONO_SM, fill=color)
        y += 20
    img.save(OUT / "10.png")
    GROUND_TRUTH["10.png"] = {
        "question": "Which container failed and why?",
        "correct_element": "farscry-api container",
        "correct_action": "farscry-api failed - port 8080 already in use, run lsof -i :8080 to find what's using it",
        "category": "terminal",
        "advantage": "tie",
    }

def make_11():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 240, 10, "kubectl get pods - farscry-prod", font=UI_MD, fill=WHITE)
    text(d, 20, 50, "NAME                           READY   STATUS              RESTARTS   AGE", font=MONO_SM, fill=DIM)
    rect(d, 0, 66, 800, 1, fill=BORDER)
    pods = [
        ("farscry-api-5d8f9c-x7k2p",    "1/1", "Running",             "0",   "2d"),
        ("farscry-api-5d8f9c-m3n8q",    "1/1", "Running",             "0",   "2d"),
        ("farscry-worker-7b4d2f-p9x1r", "0/1", "CrashLoopBackOff",    "47",  "3h"),
        ("farscry-redis-0",             "1/1", "Running",             "0",   "5d"),
        ("farscry-db-0",                "1/1", "Running",             "0",   "5d"),
        ("farscry-ingress-6c7f-n2k4m",  "1/1", "Running",             "0",   "2d"),
    ]
    y = 72
    for name, ready, status, restarts, age in pods:
        bg = "#2a1515" if status == "CrashLoopBackOff" else DARK_BG
        rect(d, 0, y, 800, 24, fill=bg)
        color = RED if status == "CrashLoopBackOff" else WHITE
        text(d, 20, y+5, f"{name:<32} {ready:<8} {status:<20} {restarts:<10} {age}", font=MONO_SM, fill=color)
        y += 25
    text(d, 20, y+15, "$ kubectl logs farscry-worker-7b4d2f-p9x1r --tail=5", font=MONO_SM, fill=GREEN)
    text(d, 20, y+33, "Error: REDIS_URL environment variable not set", font=MONO_SM, fill=RED)
    img.save(OUT / "11.png")
    GROUND_TRUTH["11.png"] = {
        "question": "Which pod is failing and what is the root cause from the logs?",
        "correct_element": "farscry-worker-7b4d2f-p9x1r",
        "correct_action": "CrashLoopBackOff (47 restarts): REDIS_URL env var not set",
        "category": "terminal",
        "advantage": "farscry",
    }

def make_12():
    img, d = new(800, 480, "#1a1a2e")
    rect(d, 0, 0, 800, 52, fill="#16213e")
    text(d, 24, 16, "React Error Boundary - localhost:3000", font=UI_MD, fill="#e0e0e0")
    rect(d, 60, 80, 680, 340, fill="#1a0a0a", outline=RED)
    text(d, 80, 104, "  Uncaught Error", font=UI_BOLD, fill=RED)
    rect(d, 80, 126, 640, 1, fill="#5a2020")
    text(d, 80, 140, "TypeError: Cannot read properties of undefined", font=MONO_MD, fill="#f48771")
    text(d, 80, 160, "          (reading 'map')", font=MONO_MD, fill="#f48771")
    text(d, 80, 188, "Call Stack:", font=UI_SM, fill=DIM)
    stack = [
        ("ProductList",   "ProductList.jsx:47"),
        ("renderWithHooks", "react-dom.development.js:14985"),
        ("mountIndeterminateComponent", "react-dom.development.js:17811"),
        ("App",           "App.jsx:23"),
    ]
    y = 206
    for fn, loc in stack:
        text(d, 80, y, fn, font=MONO_SM, fill=CYAN)
        text(d, 280, y, loc, font=MONO_SM, fill=DIM)
        y += 20
    text(d, 80, y+16, "This error happened while rendering. See the call stack above.", font=UI_SM, fill=DIM)
    bx = 560
    rect(d, bx, 380, 160, 32, fill=BTN_RED, r=4)
    text(d, bx+25, 389, "Copy stack trace", font=UI_SM, fill="#ffffff")
    img.save(OUT / "12.png")
    GROUND_TRUTH["12.png"] = {
        "question": "What is the error and which component + line caused it?",
        "correct_element": "ProductList.jsx line 47",
        "correct_action": "TypeError: Cannot read properties of undefined (reading 'map') at ProductList.jsx:47",
        "category": "browser",
        "advantage": "tie",
    }

def make_13():
    img, d = new(800, 520, LIGHT_BG)
    rect(d, 0, 0, 800, 52, fill="#6200ea")
    text(d, 24, 16, "Checkout - Step 2 of 3: Shipping", font=UI_BOLD, fill="#ffffff")
    rect(d, 40, 68, 720, 414, fill="#ffffff", outline=LIGHT_BRD)
    fields_left = [
        ("First Name", "John", False, False),
        ("Last Name", "Smith", False, False),
        ("Email", "john.smith@example.com", False, False),
    ]
    fields_right = [
        ("Phone", "+1 (555) 000-0000", False, False),
    ]
    text(d, 64, 88, "First Name *", font=UI_SM, fill=LIGHT_DIM)
    rect(d, 64, 106, 310, 36, fill="#fff", outline=LIGHT_BRD)
    text(d, 72, 115, "John", font=MONO_MD, fill=LIGHT_TXT)
    text(d, 420, 88, "Last Name *", font=UI_SM, fill=LIGHT_DIM)
    rect(d, 420, 106, 310, 36, fill="#fff", outline=LIGHT_BRD)
    text(d, 428, 115, "Smith", font=MONO_MD, fill=LIGHT_TXT)
    text(d, 64, 160, "Email *", font=UI_SM, fill=LIGHT_DIM)
    rect(d, 64, 178, 666, 36, fill="#fff", outline=LIGHT_BRD)
    text(d, 72, 187, "john.smith@example.com", font=MONO_MD, fill=LIGHT_TXT)
    text(d, 64, 232, "Address Line 1 *", font=UI_BOLD, fill="#6200ea")
    rect(d, 64, 250, 666, 36, fill="#ede7f6", outline="#6200ea")
    text(d, 72, 259, "Enter your street address", font=MONO_MD, fill="#9575cd")
    text(d, 64, 290, "This field is required", font=UI_SM, fill=LIGHT_ERR)
    text(d, 64, 315, "City *", font=UI_SM, fill=LIGHT_DIM)
    rect(d, 64, 333, 310, 36, fill="#fff", outline=LIGHT_BRD)
    text(d, 64, 387, "ZIP Code *", font=UI_SM, fill=LIGHT_DIM)
    rect(d, 64, 405, 150, 36, fill="#fff", outline=LIGHT_BRD)
    rect(d, 560, 440, 180, 38, fill=BTN_GRAY, r=4)
    text(d, 582, 449, "Continue ->", font=UI_MD, fill="#aaaaaa")
    img.save(OUT / "13.png")
    GROUND_TRUTH["13.png"] = {
        "question": "Which field is currently focused/highlighted and what must the user enter?",
        "correct_element": "Address Line 1 field (highlighted in purple)",
        "correct_action": "Address Line 1 is empty and required - user must enter their street address",
        "category": "web_form",
        "advantage": "farscry",
    }

def make_14():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 280, 10, "git merge feature/auth", font=UI_MD, fill=WHITE)
    lines = [
        ("$ git merge feature/auth",             GREEN),
        ("Auto-merging src/auth/middleware.js",  WHITE),
        ("CONFLICT (content): Merge conflict in src/auth/middleware.js", RED),
        ("Auto-merging src/utils/helpers.js",    WHITE),
        ("Automatic merge failed; fix conflicts and commit.", YELLOW),
        ("",                                     WHITE),
        ("$ cat src/auth/middleware.js | head -20", GREEN),
        ("<<<<<<< HEAD",                         RED),
        ("  if (!req.headers.authorization) {", WHITE),
        ("    return res.status(401).json({ error: 'Missing token' });", WHITE),
        ("=======",                              YELLOW),
        ("  const token = req.headers['x-auth-token'];", WHITE),
        ("  if (!token) {",                      WHITE),
        ("    return res.status(403).json({ error: 'Forbidden' });", WHITE),
        (">>>>>>> feature/auth",                 GREEN),
        ("",                                     WHITE),
        ("$ _",                                  GREEN),
    ]
    y = 50
    for line, color in lines:
        text(d, 20, y, line, font=MONO_SM, fill=color)
        y += 20
    img.save(OUT / "14.png")
    GROUND_TRUTH["14.png"] = {
        "question": "Which file has a merge conflict and what are the two conflicting changes?",
        "correct_element": "src/auth/middleware.js",
        "correct_action": "HEAD uses authorization header with 401; feature/auth uses x-auth-token with 403",
        "category": "terminal",
        "advantage": "tie",
    }

def make_15():
    img, d = new(800, 480, DARK_BG)
    rect(d, 0, 0, 800, 36, fill=DARK_BG3)
    tab_bar(d, 0, 0, ["PROBLEMS", "TERMINAL", "OUTPUT", "TEST RESULTS"], active=3)
    lines = [
        ("  ● Pipeline Tests › process › should extract text regions", RED),
        ("",                                                           WHITE),
        ("    expect(received).toHaveLength(expected)",                WHITE),
        ("",                                                           WHITE),
        ("    Expected length: 5",                                     GREEN),
        ("    Received length: 3",                                     RED),
        ("",                                                           WHITE),
        ("    at Object.<anonymous> (tests/pipeline.test.ts:47:28)",   DIM),
        ("",                                                           WHITE),
        ("  OK Pipeline Tests › build › should load models  (234ms)",  GREEN),
        ("  OK Pipeline Tests › build › should accept config  (12ms)", GREEN),
        ("  ✗ Pipeline Tests › process › should extract text regions", RED),
        ("  OK Pipeline Tests › diff  › should detect changes  (89ms)", GREEN),
        ("",                                                           WHITE),
        ("Tests:  1 failed, 3 passed, 4 total",                       RED),
        ("Suites: 1 failed, 0 passed, 1 total",                       RED),
        ("Time:   4.821s",                                            DIM),
    ]
    y = 44
    for line, color in lines:
        text(d, 16, y, line, font=MONO_SM, fill=color)
        y += 19
    img.save(OUT / "15.png")
    GROUND_TRUTH["15.png"] = {
        "question": "Which test failed and what was the assertion failure?",
        "correct_element": "Pipeline Tests › process › should extract text regions",
        "correct_action": "Expected 5 text regions, received 3 - at pipeline.test.ts:47",
        "category": "vscode",
        "advantage": "tie",
    }

def make_16():
    img, d = new(800, 520, LIGHT_BG)
    rect(d, 0, 0, 800, 52, fill="#232f3e")
    text(d, 24, 10, "AWS", font=UI_BOLD, fill="#ff9900")
    text(d, 60, 10, "Lambda › Functions › farscry-processor › Configuration", font=UI_SM, fill="#d5dbdb")
    rect(d, 0, 52, 800, 1, fill="#ff9900")
    rect(d, 20, 68, 760, 420, fill="#ffffff", outline=LIGHT_BRD)
    text(d, 40, 88, "General configuration", font=UI_BOLD, fill=LIGHT_TXT)
    configs = [
        ("Memory (MB)",        "512",           False),
        ("Timeout",            "3 min 0 sec",   False),
        ("Ephemeral storage",  "512 MB",        False),
        ("Architecture",       "arm64",         False),
        ("Concurrency",        "Unreserved",    False),
        ("Code size",          "8.2 MB",        False),
    ]
    y = 116
    for label, val, _ in configs:
        text(d, 40, y, label, font=UI_SM, fill=LIGHT_DIM)
        text(d, 260, y, val, font=UI_MD, fill=LIGHT_TXT)
        y += 32
    rect(d, 40, y+8, 1, 320, fill=LIGHT_BRD)
    text(d, 560, 116, "Runtime", font=UI_SM, fill=LIGHT_DIM)
    text(d, 560, 136, "provided.al2023", font=UI_MD, fill=LIGHT_TXT)
    text(d, 560, 168, "Handler", font=UI_SM, fill=LIGHT_DIM)
    text(d, 560, 188, "bootstrap", font=UI_MD, fill=LIGHT_TXT)
    rect(d, 560, 220, 180, 34, fill="#ffffff", outline="#0073bb", r=3)
    text(d, 578, 229, "Edit configuration", font=UI_SM, fill="#0073bb")
    rect(d, 600, 430, 160, 34, fill="#ec7211", r=3)
    text(d, 620, 439, "Save  ▸", font=UI_MD, fill="#ffffff")
    img.save(OUT / "16.png")
    GROUND_TRUTH["16.png"] = {
        "question": "What is the Memory configuration and what button saves changes?",
        "correct_element": "Memory 512MB; Save button at bottom right",
        "correct_action": "Memory is 512 MB; click orange Save button to save configuration",
        "category": "config",
        "advantage": "farscry",
    }

def make_17():
    img, d = new(800, 480, "#f8f8f8")
    rect(d, 0, 0, 800, 52, fill="#b22222")
    text(d, 24, 14, "500 INTERNAL SERVER ERROR", font=UI_BOLD, fill="#ffffff")
    text(d, 24, 80, "Traceback (most recent call last):", font=MONO_SM, fill="#333")
    stack = [
        ('  File "/app/app.py", line 42, in checkout', None),
        ('    result = db.execute(query, params)',       None),
        ('  File "/usr/lib/python3.11/sqlite3/dbapi2.py", line 67, in execute', None),
        ('    return self._execute(sql, params)',        None),
    ]
    y = 104
    for line, _ in stack:
        text(d, 24, y, line, font=MONO_SM, fill="#555")
        y += 18
    text(d, 24, y+4, "sqlalchemy.exc.OperationalError: (sqlite3.OperationalError)", font=MONO_SM, fill="#b22222")
    text(d, 24, y+22, "  no such column: orders.payment_status", font=MONO_SM, fill="#b22222")
    text(d, 24, y+48, "The above exception was the direct cause of the following exception:", font=MONO_SM, fill="#888")
    text(d, 24, y+72, "sqlalchemy.exc.OperationalError: no such column: orders.payment_status", font=MONO_MD, fill="#b22222")
    rect(d, 24, y+100, 752, 36, fill="#fff3f3", outline="#b22222")
    text(d, 36, y+110, "Run: flask db migrate && flask db upgrade  to apply pending migrations", font=MONO_SM, fill="#333")
    img.save(OUT / "17.png")
    GROUND_TRUTH["17.png"] = {
        "question": "What database error occurred and what is the fix?",
        "correct_element": "orders.payment_status column missing",
        "correct_action": "Column 'orders.payment_status' doesn't exist - run flask db migrate && flask db upgrade",
        "category": "browser",
        "advantage": "tie",
    }

def make_18():
    img, d = new(800, 480, "#cccccc")
    rect(d, 0, 0, 800, 480, fill="#00000088")
    rect(d, 180, 110, 440, 280, fill="#ffffff", outline="#e0e0e0")
    text(d, 200, 132, "  Delete Environment", font=UI_BOLD, fill="#b00020")
    rect(d, 180, 156, 440, 1, fill="#e0e0e0")
    text(d, 200, 170, "You are about to delete the production", font=UI_MD, fill=LIGHT_TXT)
    text(d, 200, 190, "environment 'farscry-prod-eu'. This action", font=UI_MD, fill=LIGHT_TXT)
    text(d, 200, 210, "cannot be undone. All data will be lost.", font=UI_MD, fill=LIGHT_TXT)
    text(d, 200, 240, "Type the environment name to confirm:", font=UI_SM, fill=LIGHT_DIM)
    rect(d, 200, 258, 360, 36, fill="#fff", outline=LIGHT_BRD)
    text(d, 208, 267, "farscry-prod-eu", font=MONO_MD, fill=LIGHT_TXT)
    rect(d, 200, 322, 120, 36, fill="#ffffff", outline=LIGHT_BRD, r=4)
    text(d, 225, 331, "Cancel", font=UI_MD, fill=LIGHT_TXT)
    rect(d, 342, 322, 80, 36, fill="#ffffff", outline=LIGHT_BRD, r=4)
    text(d, 360, 331, "Back", font=UI_MD, fill=LIGHT_TXT)
    rect(d, 440, 322, 156, 36, fill=BTN_RED, r=4)
    text(d, 455, 331, "Delete Environment", font=UI_SM, fill="#ffffff")
    img.save(OUT / "18.png")
    GROUND_TRUTH["18.png"] = {
        "question": "What is the dangerous action button and what coordinates is it at?",
        "correct_element": "Delete Environment button (red, far right)",
        "correct_action": "Red 'Delete Environment' button at approximately (440, 322) - irreversible action",
        "category": "web_form",
        "advantage": "farscry",
    }

def make_19():
    img, d = new(800, 480)
    rect(d, 0, 0, 800, 36, fill="#3c3c3c")
    text(d, 220, 10, "tail -f /var/log/nginx/access.log", font=UI_MD, fill=WHITE)
    entries = [
        ("10.0.1.5",  "POST /api/v1/extract",        "200", "142ms"),
        ("10.0.1.5",  "POST /api/v1/extract",        "200", "138ms"),
        ("10.0.1.8",  "POST /api/v1/diff",           "200", "89ms"),
        ("10.0.1.12", "GET  /api/v1/health",         "200", "3ms"),
        ("10.0.1.5",  "POST /api/v1/extract",        "500", "4023ms"),
        ("10.0.1.5",  "POST /api/v1/extract",        "500", "4001ms"),
        ("10.0.1.5",  "POST /api/v1/extract",        "500", "4189ms"),
        ("10.0.1.8",  "POST /api/v1/diff",           "502", "31ms"),
        ("10.0.1.12", "GET  /api/v1/health",         "200", "2ms"),
        ("10.0.1.5",  "POST /api/v1/extract",        "500", "4201ms"),
        ("10.0.1.15", "POST /api/v1/extract",        "500", "timeout"),
        ("10.0.1.8",  "POST /api/v1/diff",           "502", "28ms"),
    ]
    y = 48
    for ip, path, code, lat in entries:
        bg = "#2a1515" if code.startswith("5") else DARK_BG
        color = RED if code.startswith("5") else WHITE
        text(d, 16, y, f"{ip:<14} {path:<35} {code}   {lat}", font=MONO_SM, fill=color)
        y += 24
    img.save(OUT / "19.png")
    GROUND_TRUTH["19.png"] = {
        "question": "Which endpoint is returning 500 errors and what does the latency suggest?",
        "correct_element": "POST /api/v1/extract - 4 errors, 4000ms+ latency",
        "correct_action": "/api/v1/extract is timing out (~4s) causing 500s - likely model loading issue",
        "category": "terminal",
        "advantage": "farscry",
    }

def make_20():
    img, d = new(800, 520, LIGHT_BG)
    rect(d, 0, 0, 800, 52, fill="#1565c0")
    text(d, 24, 16, "Organization Settings - Security", font=UI_BOLD, fill="#ffffff")
    rect(d, 0, 52, 800, 3, fill="#ffa000")
    text(d, 24, 60, "● You have unsaved changes", font=UI_SM, fill="#e65100")
    rect(d, 20, 80, 760, 400, fill="#ffffff", outline=LIGHT_BRD)
    toggles = [
        ("Require 2FA for all members", True, "Members must use two-factor authentication"),
        ("Allow SSH key access",         True, "Members can use SSH keys to clone repositories"),
        ("SAML single sign-on",          False, "Not configured - requires Business plan"),
        ("IP allowlist",                 False, "Restrict access to specific IP ranges"),
    ]
    y = 100
    for label, enabled, desc in toggles:
        status_color = BTN_GREEN if enabled else BTN_GRAY
        status_text = "ON" if enabled else "OFF"
        rect(d, 40, y, 40, 22, fill=status_color, r=11)
        text(d, 52, y+4, status_text, font=UI_SM, fill="#ffffff")
        text(d, 96, y, label, font=UI_BOLD, fill=LIGHT_TXT)
        text(d, 96, y+18, desc, font=UI_SM, fill=LIGHT_DIM)
        y += 52
    rect(d, 0, 460, 800, 3, fill="#ffa000")
    rect(d, 20, 468, 760, 52, fill="#fff8e1", outline="#ffe082")
    rect(d, 576, 478, 90, 34, fill=BTN_BLUE, r=4)
    text(d, 596, 487, "Save", font=UI_BOLD, fill="#ffffff")
    rect(d, 676, 478, 90, 34, fill="#ffffff", outline=LIGHT_BRD, r=4)
    text(d, 691, 487, "Discard", font=UI_MD, fill=LIGHT_TXT)
    img.save(OUT / "20.png")
    GROUND_TRUTH["20.png"] = {
        "question": "What settings are currently enabled and where is the Save button?",
        "correct_element": "Require 2FA (ON) + SSH key access (ON); Save button at (576, 478)",
        "correct_action": "2FA and SSH access are ON; SAML and IP allowlist OFF; Save button is blue at bottom right",
        "category": "config",
        "advantage": "farscry",
    }

if __name__ == "__main__":
    print("Generating 20 benchmark screenshots...")
    for i, fn in enumerate([
        make_01, make_02, make_03, make_04, make_05,
        make_06, make_07, make_08, make_09, make_10,
        make_11, make_12, make_13, make_14, make_15,
        make_16, make_17, make_18, make_19, make_20,
    ], 1):
        fn()
        print(f"  {i:02d}/20 done")

    gt_path = Path(__file__).parent / "ground_truth.json"
    gt_path.write_text(json.dumps(GROUND_TRUTH, indent=2))
    print(f"\nGround truth saved to {gt_path}")
    print(f"Screenshots saved to {OUT}/")

    advantages = {}
    for v in GROUND_TRUTH.values():
        a = v["advantage"]
        advantages[a] = advantages.get(a, 0) + 1
    print(f"\nDesigned advantage distribution:")
    for k, v in sorted(advantages.items()):
        print(f"  {k}: {v} screenshots")
    print("\nDone.")
