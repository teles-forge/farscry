"""
Generate synthetic UI screenshots for the farscry diff spike.

Test 1 - Scroll:  same content at different Y positions (scroll down ~240px)
Test 2 - Field:   same form, fields go from placeholder -> real value
Test 3 - Error:   error banner appears; all other elements stay put
"""

from PIL import Image, ImageDraw, ImageFont
import os


FONT_PATH = "/System/Library/Fonts/Supplemental/Arial.ttf"
IMG_W     = 1000
MARGIN_X  = 60

WHITE     = (255, 255, 255)
BLACK     = (15,  15,  15)
DARK_GRAY = (60,  60,  60)
GRAY      = (130, 130, 130)
LIGHT_RED = (255, 220, 220)
DARK_RED  = (160, 0,   0)
BTN_BG    = (225, 225, 225)
BTN_BORDER= (110, 110, 110)
FIELD_BORDER_EMPTY  = (180, 180, 180)
FIELD_BORDER_FILLED = (60,  100, 200)

def make_font(size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(FONT_PATH, size)


def make_image(rows: list[dict]) -> Image.Image:
    """
    rows: list of {text, y, style}
    style: 'title' | 'body' | 'label' | 'input' | 'input_filled' | 'button' | 'error'
    """
    max_y = max(r["y"] for r in rows) + 60
    h = max(max_y + 40, 200)
    img = Image.new("RGB", (IMG_W, h), WHITE)
    draw = ImageDraw.Draw(img)

    for r in rows:
        text  = r["text"]
        y     = r["y"]
        style = r.get("style", "body")

        if style == "title":
            f = make_font(32)
            draw.text((MARGIN_X, y), text, fill=BLACK, font=f)

        elif style in ("body", "label"):
            f = make_font(26)
            fill = BLACK if style == "body" else DARK_GRAY
            draw.text((MARGIN_X, y), text, fill=fill, font=f)

        elif style == "input":
            f = make_font(26)
            bb = draw.textbbox((MARGIN_X, y), text, font=f)
            field_r = max(bb[2] + 60, MARGIN_X + 420)
            draw.rectangle(
                [MARGIN_X - 8, y - 5, field_r, bb[3] + 8],
                outline=FIELD_BORDER_EMPTY, width=2
            )
            draw.text((MARGIN_X, y), text, fill=GRAY, font=f)

        elif style == "input_filled":
            f = make_font(26)
            bb = draw.textbbox((MARGIN_X, y), text, font=f)
            field_r = max(bb[2] + 60, MARGIN_X + 420)
            draw.rectangle(
                [MARGIN_X - 8, y - 5, field_r, bb[3] + 8],
                outline=FIELD_BORDER_FILLED, width=2
            )
            draw.text((MARGIN_X, y), text, fill=BLACK, font=f)

        elif style == "button":
            f = make_font(26)
            bb = draw.textbbox((MARGIN_X, y), text, font=f)
            draw.rectangle(
                [bb[0] - 12, bb[1] - 8, bb[2] + 12, bb[3] + 8],
                fill=BTN_BG, outline=BTN_BORDER, width=1
            )
            draw.text((MARGIN_X, y), text, fill=BLACK, font=f)

        elif style == "error":
            f = make_font(26)
            bb = draw.textbbox((MARGIN_X, y), text, font=f)
            draw.rectangle(
                [0, y - 10, IMG_W, bb[3] + 12],
                fill=LIGHT_RED
            )
            draw.text((MARGIN_X, y), text, fill=DARK_RED, font=f)

    return img


Y0, DY = 30, 60

BEFORE_1 = [
    {"text": "Dashboard",              "y": Y0 + 0*DY, "style": "title"},
    {"text": "Welcome back Admin",     "y": Y0 + 1*DY, "style": "body"},
    {"text": "Total Users: 1234",      "y": Y0 + 2*DY, "style": "body"},
    {"text": "Active Sessions: 42",    "y": Y0 + 3*DY, "style": "body"},
    {"text": "Revenue: $9876",         "y": Y0 + 4*DY, "style": "body"},
    {"text": "Recent Activity",        "y": Y0 + 5*DY, "style": "body"},
    {"text": "User login 9:00 AM",     "y": Y0 + 6*DY, "style": "body"},
    {"text": "Report 9:15 AM",         "y": Y0 + 7*DY, "style": "body"},
    {"text": "Settings 9:30 AM",       "y": Y0 + 8*DY, "style": "body"},
    {"text": "New Orders: 17",         "y": Y0 + 9*DY, "style": "body"},
]

AFTER_1 = [
    {"text": "Revenue: $9876",         "y": Y0 + 0*DY, "style": "body"},
    {"text": "Recent Activity",        "y": Y0 + 1*DY, "style": "body"},
    {"text": "User login 9:00 AM",     "y": Y0 + 2*DY, "style": "body"},
    {"text": "Report 9:15 AM",         "y": Y0 + 3*DY, "style": "body"},
    {"text": "Settings 9:30 AM",       "y": Y0 + 4*DY, "style": "body"},
    {"text": "New Orders: 17",         "y": Y0 + 5*DY, "style": "body"},
    {"text": "API calls: 5432",        "y": Y0 + 6*DY, "style": "body"},
    {"text": "Storage used: 78%",      "y": Y0 + 7*DY, "style": "body"},
    {"text": "Pending tasks: 3",       "y": Y0 + 8*DY, "style": "body"},
    {"text": "Last backup: 10:00 AM",  "y": Y0 + 9*DY, "style": "body"},
]


BEFORE_2 = [
    {"text": "User Registration",  "y":  30, "style": "title"},
    {"text": "First Name:",        "y": 110, "style": "label"},
    {"text": "Enter your name",    "y": 148, "style": "input"},
    {"text": "Email Address:",     "y": 220, "style": "label"},
    {"text": "Enter your email",   "y": 258, "style": "input"},
    {"text": "Submit",             "y": 360, "style": "button"},
]

AFTER_2 = [
    {"text": "User Registration",  "y":  30, "style": "title"},
    {"text": "First Name:",        "y": 110, "style": "label"},
    {"text": "John Smith",         "y": 148, "style": "input_filled"},
    {"text": "Email Address:",     "y": 220, "style": "label"},
    {"text": "john.doe@test.com",  "y": 258, "style": "input_filled"},
    {"text": "Submit",             "y": 360, "style": "button"},
]


BEFORE_3 = [
    {"text": "Payment Portal",              "y":  80, "style": "title"},
    {"text": "Card: 4242 4242 4242 4242",   "y": 160, "style": "body"},
    {"text": "Expiry: 12/26",               "y": 220, "style": "body"},
    {"text": "Amount: $99.00",              "y": 280, "style": "body"},
    {"text": "Pay Now",                      "y": 380, "style": "button"},
]

AFTER_3 = [
    {"text": "ERROR: Payment declined",     "y":  15, "style": "error"},
    {"text": "Payment Portal",              "y":  80, "style": "title"},
    {"text": "Card: 4242 4242 4242 4242",   "y": 160, "style": "body"},
    {"text": "Expiry: 12/26",               "y": 220, "style": "body"},
    {"text": "Amount: $99.00",              "y": 280, "style": "body"},
    {"text": "Pay Now",                      "y": 380, "style": "button"},
]


BASE = os.path.dirname(os.path.abspath(__file__))

IMAGES = [
    ("test1_scroll/before.png", BEFORE_1),
    ("test1_scroll/after.png",  AFTER_1),
    ("test2_field/before.png",  BEFORE_2),
    ("test2_field/after.png",   AFTER_2),
    ("test3_error/before.png",  BEFORE_3),
    ("test3_error/after.png",   AFTER_3),
]

for rel_path, rows in IMAGES:
    out = os.path.join(BASE, rel_path)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    img = make_image(rows)
    img.save(out)
    print(f"OK {rel_path}  ({img.width}x{img.height})")

print("\nAll images generated.")
