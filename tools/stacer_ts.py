"""Read Stacer's Qt Linguist (.ts) translations and match them to Kaptan's msgids."""
import re
import xml.etree.ElementTree as ET


def norm(s):
    """Key used to match a Kaptan msgid with a Stacer source text."""
    s = s.replace("{}", "%1")
    s = re.sub(r"%\d", "%1", s)
    s = s.replace("…", "...").strip().rstrip(".:!?").strip()
    return s.lower()


def to_kaptan(text, msgid):
    """Stacer translation -> Kaptan placeholders; None if they don't line up."""
    n = msgid.count("{}")
    out = text
    for i in range(1, 10):
        out = out.replace(f"%{i}", "{}")
    if out.count("{}") != n:
        return None
    # keep Kaptan's trailing punctuation style
    if msgid.endswith(".") and not out.endswith((".", "。")):
        out += "."
    if not msgid.endswith((".", ":", "…")) and out.endswith(".") and not msgid.endswith("."):
        out = out[:-1]
    return out


def load(path):
    """{normalized source: translation} of all non-empty translations."""
    tree = ET.parse(path)
    out = {}
    for msg in tree.iter("message"):
        src = msg.findtext("source") or ""
        tr = msg.find("translation")
        if tr is None or not (tr.text or "").strip():
            continue
        if tr.get("type") == "unfinished" and (tr.text or "").strip() == src.strip():
            continue  # placeholder copies of the English text
        out.setdefault(norm(src), tr.text.strip())
    return out
