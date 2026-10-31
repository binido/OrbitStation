"""Renders REA inspect-managed-members JSON as readable IL: il.py <json> <type-substring> [method]"""
import json, sys

n = json.load(open(sys.argv[1]))["normalized_result"]
names = {}
for m in n["methods"]:
    names[m["token"]] = m["declaring_type"].split(".")[-1] + "::" + m["name"]
for m in n["member_refs"]:
    names[m["token"]] = (m.get("parent_name") or "?").split(".")[-1] + "::" + m["name"]
for f in n["fields"]:
    names[f["token"]] = f["declaring_type"].split(".")[-1] + "." + f["name"]
for t in n["types"]:
    names[t["token"]] = t["full_name"]
for s in n["user_strings"]["literals"]:
    names[s["token"]] = json.dumps(s["value"], ensure_ascii=False)

want = sys.argv[2]
meth = sys.argv[3] if len(sys.argv) > 3 else None
for m in n["methods"]:
    if want not in m["declaring_type"] or (meth and m["name"] != meth):
        continue
    b = m.get("body") or {}
    print("\n## %s::%s (%s) -> %s" % (m["declaring_type"], m["name"],
          ",".join(m["signature"].get("parameter_types") or []), m["signature"].get("return_type")))
    for a in b.get("anchors") or []:
        op = a["operand"]
        if a["operand_kind"] in ("method", "field", "type", "string", "token"):
            op = names.get(op, op)
        print("  %4s %-12s %s" % (a["il_offset"], a["opcode"], "" if op is None else op))
