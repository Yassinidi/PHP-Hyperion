<?php
// Class constants and file-scope `const`. Run with:
//   ./target/release/hyperion-cli tests/unit/constants.php
//
// Every assertion here failed before task #14: `const X = ...` was parsed as an
// assignment to a local named X and thrown away, so `self::COLORS` in Symfony's
// Color class read back null and `array_keys(self::COLORS)` was the first thing
// to notice.

function check($label, $got, $want) {
    if ($got === $want) {
        echo "PASS " . $label . "\n";
    } else {
        echo "FAIL " . $label . " got=" . var_export($got, true)
           . " want=" . var_export($want, true) . "\n";
    }
}

// ------------------------------------------------------- file-scope constants
const TOP_SCALAR = 5;
const TOP_STR = 'hi';
const TOP_A = 1, TOP_B = 2;
const TOP_ARRAY = ['a' => 1, 'b' => 2];

check("file-scope int", TOP_SCALAR, 5);
check("file-scope string", TOP_STR, 'hi');
check("comma list first", TOP_A, 1);
check("comma list second", TOP_B, 2);
check("file-scope array", TOP_ARRAY, ['a' => 1, 'b' => 2]);
check("file-scope const is defined()", defined('TOP_SCALAR'), true);
check("file-scope const via constant()", constant('TOP_STR'), 'hi');

// A `const` is global, so a function body sees it without `global`.
function readsTopConst() { return TOP_SCALAR; }
check("visible inside a function", readsTopConst(), 5);

// ------------------------------------------------------------ class constants
class K {
    const SCALAR = 5;
    const STR = 'hi';
    const EMPTY_ARR = [];
    private const COLORS = ['black' => 0, 'red' => 1];
    protected const OPTS = ['bold' => ['set' => 1, 'unset' => 22]];
    public const MULTI_A = 1, MULTI_B = 2;

    public static function viaSelf() { return self::COLORS; }
    public static function issetSelf($k) { return isset(self::COLORS[$k]); }
    public static function idxSelf($k) { return self::COLORS[$k]; }
    public static function keysSelf() { return array_keys(self::COLORS); }
    public static function nestedSelf() { return self::OPTS['bold']['set']; }
    public static function viaStaticKeyword() { return static::SCALAR; }
}

check("scalar via class name", K::SCALAR, 5);
check("string via class name", K::STR, 'hi');
check("empty array const", K::EMPTY_ARR, []);
check("comma list in class (first)", K::MULTI_A, 1);
check("comma list in class (second)", K::MULTI_B, 2);
check("array via self::", K::viaSelf(), ['black' => 0, 'red' => 1]);
check("isset on a hit", K::issetSelf('red'), true);
check("isset on a miss", K::issetSelf('nope'), false);
check("index a const array", K::idxSelf('red'), 1);
// This is the exact call that stopped Laravel booting.
check("array_keys(self::CONST)", K::keysSelf(), ['black', 'red']);
check("nested const array", K::nestedSelf(), 1);
check("static:: reads the const", K::viaStaticKeyword(), 5);

// ------------------------------------------------------- inheritance of consts
class Base { const A = 1; const SHADOWED = 'base'; }
class Child extends Base { const SHADOWED = 'child'; }

check("inherited const", Child::A, 1);
check("own const still reachable", Base::A, 1);
check("child shadows parent", Child::SHADOWED, 'child');
check("parent keeps its own", Base::SHADOWED, 'base');

class GrandChild extends Child {}
check("const two levels up", GrandChild::A, 1);
check("nearest ancestor wins", GrandChild::SHADOWED, 'child');

// -------------------------------------------------------- interface constants
interface HasVersion { const VERSION = 3; }
class Versioned implements HasVersion {}
check("const from interface", Versioned::VERSION, 3);
check("const on the interface itself", HasVersion::VERSION, 3);

interface Overridable { const WHO = 'iface'; }
class Overrides implements Overridable { const WHO = 'class'; }
check("class overrides interface const", Overrides::WHO, 'class');

// ------------------------------------------------------------ trait constants
trait Flagged { const FLAG = 'on'; }
class UsesFlag { use Flagged; }
check("const from trait", UsesFlag::FLAG, 'on');

// ------------------------------------------- ::class is not a stored constant
check("::class on a class", K::class, 'K');

// ----------------------------------------------------- static props unaffected
// Constants share the static table, so this checks the two do not collide.
class Mixed {
    const C = 'const';
    public static $s = 'static';
}
check("const beside a static prop", Mixed::C, 'const');
check("static prop beside a const", Mixed::$s, 'static');
Mixed::$s = 'written';
check("static prop still writable", Mixed::$s, 'written');
check("write did not touch the const", Mixed::C, 'const');

echo "\n-- end of suite --\n";
