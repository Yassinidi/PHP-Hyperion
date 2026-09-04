<?php
namespace Composer\Autoload;

class ClassLoader {
    private array $prefixLengthsPsr4 = [];
    private array $prefixDirsPsr4 = [];

    public function addPsr4(string $prefix, array|string $paths, bool $prepend = false): void {
        $paths = (array)$paths;
        $length = strlen($prefix);
        if ($prefix[$length - 1] !== "\\") {
            $prefix .= "\\";
            $length++;
        }
        $firstChar = $prefix[0];
        if (!isset($this->prefixDirsPsr4[$firstChar][$prefix])) {
            $this->prefixLengthsPsr4[$firstChar][$prefix] = $length;
            $this->prefixDirsPsr4[$firstChar][$prefix] = $paths;
        } else {
            $this->prefixDirsPsr4[$firstChar][$prefix] = array_merge($this->prefixDirsPsr4[$firstChar][$prefix], $paths);
        }
    }

    public function loadClass(string $class): ?bool {
        if ($file = $this->findFile($class)) {
            require $file;
            return true;
        }
        return null;
    }

    public function findFile(string $class): string|false {
        $firstChar = $class[0];
        if (isset($this->prefixLengthsPsr4[$firstChar])) {
            foreach ($this->prefixLengthsPsr4[$firstChar] as $prefix => $length) {
                if (str_starts_with($class, $prefix)) {
                    $subPath = substr($class, $length);
                    $relPath = str_replace("\\", "/", $subPath) . ".php";
                    foreach ($this->prefixDirsPsr4[$firstChar][$prefix] as $dir) {
                        $file = "{$dir}/{$relPath}";
                        if (file_exists($file)) {
                            return $file;
                        }
                    }
                }
            }
        }
        return false;
    }

    public function register(bool $prepend = false): void {
        spl_autoload_register([$this, "loadClass"], true, $prepend);
    }
}
