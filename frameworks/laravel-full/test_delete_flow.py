import urllib.request
import re
import json
import html as html_module

# Session cookies container
cookies = {}

def get_cookie_header():
    return "; ".join([f"{k}={v}" for k, v in cookies.items()])

def update_cookies(resp):
    for c in resp.headers.get_all("Set-Cookie") or []:
        kv = c.split(";")[0].split("=")
        cookies[kv[0]] = kv[1]

# 1. GET /admin/login
req = urllib.request.Request("http://127.0.0.1:8000/admin/login")
with urllib.request.urlopen(req) as resp:
    update_cookies(resp)
    html = resp.read().decode('utf-8')

csrf_token = re.search(r'data-csrf="([^"]+)"', html).group(1)
snap_match = re.search(r'wire:snapshot="([^"]+)"', html)
snapshot = json.loads(html_module.unescape(snap_match.group(1)))

# 2. Authenticate
login_payload = {
    "_token": csrf_token,
    "components": [
        {
            "snapshot": json.dumps(snapshot),
            "updates": {"data.email": "admin@example.com", "data.password": "password"},
            "calls": [{"path": "", "method": "authenticate", "params": []}]
        }
    ]
}

post_req = urllib.request.Request(
    "http://127.0.0.1:8000/livewire/update",
    data=json.dumps(login_payload).encode('utf-8'),
    method="POST"
)
post_req.add_header("Cookie", get_cookie_header())
post_req.add_header("Content-Type", "application/json")
post_req.add_header("X-Livewire", "true")
post_req.add_header("X-CSRF-TOKEN", csrf_token)

with urllib.request.urlopen(post_req) as post_resp:
    update_cookies(post_resp)
    print("1. Login status:", post_resp.status)

# 3. GET /admin/categories/5/edit
edit_req = urllib.request.Request("http://127.0.0.1:8000/admin/categories/5/edit")
edit_req.add_header("Cookie", get_cookie_header())

try:
    with urllib.request.urlopen(edit_req) as edit_resp:
        update_cookies(edit_resp)
        print("2. GET /admin/categories/5/edit status:", edit_resp.status)
        edit_html = edit_resp.read().decode('utf-8')
except urllib.error.HTTPError as e:
    print("2. GET edit failed:", e.code, e.headers.get("Location"))
    exit(1)

# Extract EditCategory snapshot
csrf_token2 = re.search(r'data-csrf="([^"]+)"', edit_html).group(1)
# Find the component with EditCategory
edit_snap_match = re.findall(r'wire:snapshot="([^"]+)"', edit_html)
edit_snapshot = None
for s in edit_snap_match:
    parsed = json.loads(html_module.unescape(s))
    if "Category" in parsed.get("memo", {}).get("name", ""):
        edit_snapshot = parsed
        break

if not edit_snapshot:
    print("Could not find EditCategory snapshot")
    exit(1)

print("3. Found component:", edit_snapshot.get("memo", {}).get("name"))

# 4. Call mountAction('delete') then callMountedAction()
delete_payload = {
    "_token": csrf_token2,
    "components": [
        {
            "snapshot": json.dumps(edit_snapshot),
            "updates": {},
            "calls": [
                {"path": "", "method": "mountAction", "params": ["delete"]},
                {"path": "", "method": "callMountedAction", "params": []}
            ]
        }
    ]
}

del_req = urllib.request.Request(
    "http://127.0.0.1:8000/livewire/update",
    data=json.dumps(delete_payload).encode('utf-8'),
    method="POST"
)
del_req.add_header("Cookie", get_cookie_header())
del_req.add_header("Content-Type", "application/json")
del_req.add_header("X-Livewire", "true")
del_req.add_header("X-CSRF-TOKEN", csrf_token2)

with urllib.request.urlopen(del_req) as del_resp:
    update_cookies(del_resp)
    del_json = json.loads(del_resp.read().decode('utf-8'))
    print("4. Delete response status:", del_resp.status)
    effects = del_json.get("components", [{}])[0].get("effects", {})
    redirect_target = effects.get("redirect")
    print("5. Effects redirect target:", redirect_target)

# 5. Follow the redirect target!
if redirect_target:
    follow_req = urllib.request.Request(redirect_target)
    follow_req.add_header("Cookie", get_cookie_header())
    try:
        with urllib.request.urlopen(follow_req) as follow_resp:
            print("6. Follow redirect status:", follow_resp.status)
            print("7. Follow redirect final URL:", follow_resp.geturl())
            if "/login" in follow_resp.geturl():
                print("--> REDIRECTED TO LOGIN DETECTED!")
            else:
                print("--> REMAINED AUTHENTICATED on:", follow_resp.geturl())
    except urllib.error.HTTPError as e:
        print("Follow redirect HTTP error:", e.code, e.headers.get("Location"))

