const fs = require("fs");
const html = fs.readFileSync("users_create_hyperion.html", "utf8");

// Extract all attributes like x-data="...", x-show="...", etc.
// Note: attribute values might span multiple lines or contain nested quotes.
// Use a robust regex or state machine
const regex = /\b(x-[a-zA-Z0-9.:_-]+|:[a-zA-Z0-9_-]+|@[a-zA-Z0-9.:_-]+)="([\s\S]*?)"/g;
let match;
let count = 0;
let errors = [];

while ((match = regex.exec(html)) !== null) {
    const attr = match[1];
    let val = match[2];
    count++;
    
    // HTML decode entities
    val = val.replace(/&#039;/g, "'").replace(/&quot;/g, '"').replace(/&amp;/g, "&").replace(/&lt;/g, "<").replace(/&gt;/g, ">");
    
    try {
        const AsyncFunction = Object.getPrototypeOf(async function() {}).constructor;
        let rightSideSafeExpression = /^[\n\s]*if.*\(.*\)/.test(val.trim()) || /^(let|const)\s/.test(val.trim()) ? `(async()=>{ ${val} })()` : val;
        new AsyncFunction(["__self", "scope"], `with (scope) { __self.result = ${rightSideSafeExpression} }; __self.finished = true; return __self.result;`);
    } catch (e) {
        errors.push({ attr, val: val.trim(), err: e.message });
    }
}

console.log("Total directives tested:", count);
console.log("Errors count:", errors.length);
errors.forEach((e, i) => {
    console.log(`\n--- ERROR ${i + 1} on ${e.attr} ---`);
    console.log("Message:", e.err);
    console.log("Value:", e.val);
});
