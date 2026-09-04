<?php
// SPL classes that are ordinary PHP once the engine has objects, arrays and
// interfaces. Compiled — not executed — during engine start-up; class
// definitions are registered by the compiler, so nothing here runs at boot.
//
// Keep this file to plain, widely-supported PHP: it is compiled by the same
// front end as user code, so a feature that is broken for user code is broken
// here too, and a failure at boot is far harder to read than one in a script.

class ArrayIterator implements Iterator, Countable, ArrayAccess
{
    private $storage;
    private $keys;
    private $pos = 0;

    public function __construct($array = [])
    {
        if (is_object($array)) {
            $array = get_object_vars($array);
        } else if (!is_array($array)) {
            $array = [];
        }
        $this->storage = $array;
        $this->keys = array_keys($array);
    }

    public function current(): mixed
    {
        $k = $this->key();
        if ($k === null) {
            return null;
        }
        return $this->storage[$k];
    }

    public function key(): mixed
    {
        if ($this->pos < 0 || $this->pos >= count($this->keys)) {
            return null;
        }
        return $this->keys[$this->pos];
    }

    public function next(): void
    {
        $this->pos = $this->pos + 1;
    }

    public function rewind(): void
    {
        // Re-read the keys: an offsetSet between two walks may have added some.
        $storage = is_object($this->storage) ? get_object_vars($this->storage) : $this->storage;
        $this->keys = is_array($storage) ? array_keys($storage) : [];
        $this->pos = 0;
    }

    public function valid(): bool
    {
        return $this->pos < count($this->keys);
    }

    public function count(): int
    {
        return count($this->storage);
    }

    public function offsetExists($offset): bool
    {
        return isset($this->storage[$offset]);
    }

    public function offsetGet($offset): mixed
    {
        return $this->storage[$offset];
    }

    public function offsetSet($offset, $value): void
    {
        if ($offset === null) {
            $this->storage[] = $value;
        } else {
            $this->storage[$offset] = $value;
        }
        $this->keys = array_keys($this->storage);
    }

    public function offsetUnset($offset): void
    {
        unset($this->storage[$offset]);
        $this->keys = array_keys($this->storage);
    }

    public function append($value): void
    {
        $this->storage[] = $value;
        $this->keys = array_keys($this->storage);
    }

    public function getArrayCopy()
    {
        return $this->storage;
    }
}

class RecursiveArrayIterator extends ArrayIterator implements RecursiveIterator
{
    public const CHILD_ARRAYS_ONLY = 1;

    public function hasChildren(): bool
    {
        return is_array($this->current()) || ($this->current() instanceof Traversable);
    }

    public function getChildren(): ?RecursiveArrayIterator
    {
        $curr = $this->current();
        if (is_array($curr)) {
            return new RecursiveArrayIterator($curr);
        }
        if ($curr instanceof Traversable) {
            return new RecursiveArrayIterator(iterator_to_array($curr));
        }
        return null;
    }
}

class ArrayObject implements IteratorAggregate, Countable, ArrayAccess
{
    private $storage;

    public function __construct($array = [])
    {
        $this->storage = $array;
    }

    public function append($value): void
    {
        $this->storage[] = $value;
    }

    public function getArrayCopy(): array
    {
        return is_array($this->storage) ? $this->storage : (is_object($this->storage) ? get_object_vars($this->storage) : []);
    }

    public function getIterator(): Iterator
    {
        return new ArrayIterator($this->storage);
    }

    public function count(): int
    {
        return count($this->storage);
    }

    public function offsetExists($offset): bool
    {
        return isset($this->storage[$offset]);
    }

    public function offsetGet($offset): mixed
    {
        return $this->storage[$offset];
    }

    public function offsetSet($offset, $value): void
    {
        if ($offset === null) {
            $this->storage[] = $value;
        } else {
            $this->storage[$offset] = $value;
        }
    }

    public function offsetUnset($offset): void
    {
        unset($this->storage[$offset]);
    }

    public function getArrayCopy()
    {
        return $this->storage;
    }

    public function append($value)
    {
        $this->storage[] = $value;
    }
}

class SplObjectStorage implements Countable, Iterator, Serializable, ArrayAccess
{
    private array $storage = [];
    private array $objects = [];
    private int $position = 0;

    public function attach(object $object, mixed $info = null): void
    {
        $hash = $this->getHash($object);
        $this->storage[$hash] = $info;
        $this->objects[$hash] = $object;
    }

    public function detach(object $object): void
    {
        $hash = $this->getHash($object);
        unset($this->storage[$hash], $this->objects[$hash]);
    }

    public function contains(object $object): bool
    {
        $hash = $this->getHash($object);
        return array_key_exists($hash, $this->storage);
    }

    public function addAll(SplObjectStorage $storage): int
    {
        $count = 0;
        foreach ($storage as $obj) {
            $this->attach($obj, $storage->getInfo());
            $count++;
        }
        return $count;
    }

    public function removeAll(SplObjectStorage $storage): int
    {
        $count = 0;
        foreach ($storage as $obj) {
            if ($this->contains($obj)) {
                $this->detach($obj);
                $count++;
            }
        }
        return $count;
    }

    public function removeAllExcept(SplObjectStorage $storage): int
    {
        $count = 0;
        foreach (array_values($this->objects) as $obj) {
            if (!$storage->contains($obj)) {
                $this->detach($obj);
                $count++;
            }
        }
        return $count;
    }

    public function getInfo(): mixed
    {
        $keys = array_keys($this->objects);
        if (isset($keys[$this->position])) {
            return $this->storage[$keys[$this->position]];
        }
        return null;
    }

    public function setInfo(mixed $info): void
    {
        $keys = array_keys($this->objects);
        if (isset($keys[$this->position])) {
            $this->storage[$keys[$this->position]] = $info;
        }
    }

    public function getHash(object $object): string
    {
        return spl_object_hash($object);
    }

    public function current(): mixed
    {
        $keys = array_keys($this->objects);
        if (isset($keys[$this->position])) {
            return $this->objects[$keys[$this->position]];
        }
        return null;
    }

    public function key(): int
    {
        return $this->position;
    }

    public function next(): void
    {
        $this->position++;
    }

    public function rewind(): void
    {
        $this->position = 0;
    }

    public function valid(): bool
    {
        $keys = array_keys($this->objects);
        return isset($keys[$this->position]);
    }

    public function count(): int
    {
        return count($this->storage);
    }

    public function offsetExists(mixed $object): bool
    {
        return is_object($object) && $this->contains($object);
    }

    public function offsetGet(mixed $object): mixed
    {
        if (is_object($object)) {
            $hash = $this->getHash($object);
            return $this->storage[$hash] ?? null;
        }
        return null;
    }

    public function offsetSet(mixed $object, mixed $info = null): void
    {
        if (is_object($object)) {
            $this->attach($object, $info);
        }
    }

    public function offsetUnset(mixed $object): void
    {
        if (is_object($object)) {
            $this->detach($object);
        }
    }

    public function serialize(): string
    {
        return serialize($this->__serialize());
    }

    public function unserialize(string $data): void
    {
        $this->__unserialize(unserialize($data));
    }

    public function __serialize(): array
    {
        $data = [];
        foreach ($this->objects as $hash => $obj) {
            $data[] = ['obj' => $obj, 'inf' => $this->storage[$hash]];
        }
        return $data;
    }

    public function __unserialize(array $data): void
    {
        $this->storage = [];
        $this->objects = [];
        $this->position = 0;
        foreach ($data as $item) {
            $this->attach($item['obj'], $item['inf'] ?? null);
        }
    }
}

class SplPriorityQueue implements Iterator, Countable
{
    public const EXTR_BOTH = 3;
    public const EXTR_PRIORITY = 2;
    public const EXTR_DATA = 1;

    protected array $data = [];
    protected int $flags = self::EXTR_DATA;
    protected int $index = 0;

    public function compare(mixed $priority1, mixed $priority2): int
    {
        if ($priority1 === $priority2) return 0;
        return $priority1 < $priority2 ? -1 : 1;
    }

    public function count(): int
    {
        return count($this->data);
    }

    public function current(): mixed
    {
        if (!isset($this->data[$this->index])) return null;
        $item = $this->data[$this->index];
        if ($this->flags === self::EXTR_BOTH) {
            return ['data' => $item['data'], 'priority' => $item['priority']];
        }
        if ($this->flags === self::EXTR_PRIORITY) {
            return $item['priority'];
        }
        return $item['data'];
    }

    public function extract(): mixed
    {
        if (empty($this->data)) {
            throw new RuntimeException("Can't extract from an empty queue");
        }
        $item = array_shift($this->data);
        if ($this->flags === self::EXTR_BOTH) {
            return ['data' => $item['data'], 'priority' => $item['priority']];
        }
        if ($this->flags === self::EXTR_PRIORITY) {
            return $item['priority'];
        }
        return $item['data'];
    }

    public function getExtractFlags(): int
    {
        return $this->flags;
    }

    public function insert(mixed $value, mixed $priority): true
    {
        $entry = ['data' => $value, 'priority' => $priority];
        $inserted = false;
        $total = count($this->data);
        for ($i = 0; $i < $total; $i++) {
            if ($this->compare($priority, $this->data[$i]['priority']) > 0) {
                array_splice($this->data, $i, 0, [$entry]);
                $inserted = true;
                break;
            }
        }
        if (!$inserted) {
            $this->data[] = $entry;
        }
        return true;
    }

    public function isCorrupted(): bool { return false; }
    public function isEmpty(): bool { return empty($this->data); }
    public function key(): mixed { return $this->index; }
    public function next(): void { $this->index++; }
    public function recoverFromCorruption(): bool { return true; }
    public function rewind(): void { $this->index = 0; }
    public function setExtractFlags(int $flags): int
    {
        $old = $this->flags;
        $this->flags = $flags;
        return $old;
    }
    public function top(): mixed
    {
        if (empty($this->data)) {
            throw new RuntimeException("Can't peek at an empty queue");
        }
        $item = $this->data[0];
        if ($this->flags === self::EXTR_BOTH) {
            return ['data' => $item['data'], 'priority' => $item['priority']];
        }
        if ($this->flags === self::EXTR_PRIORITY) {
            return $item['priority'];
        }
        return $item['data'];
    }
    public function valid(): bool
    {
        return isset($this->data[$this->index]);
    }
}

class SplDoublyLinkedList implements Iterator, ArrayAccess, Countable
{
    public const IT_MODE_LIFO = 2;
    public const IT_MODE_FIFO = 0;
    public const IT_MODE_DELETE = 1;
    public const IT_MODE_KEEP = 0;

    protected array $data = [];
    protected int $index = 0;
    protected int $flags = 0;

    public function bottom(): mixed
    {
        if (empty($this->data)) throw new RuntimeException("Can't peek at an empty datastructure");
        return $this->data[0];
    }
    public function top(): mixed
    {
        if (empty($this->data)) throw new RuntimeException("Can't peek at an empty datastructure");
        return $this->data[count($this->data) - 1];
    }
    public function pop(): mixed
    {
        if (empty($this->data)) throw new RuntimeException("Can't pop from an empty datastructure");
        return array_pop($this->data);
    }
    public function push(mixed $value): void
    {
        $this->data[] = $value;
    }
    public function shift(): mixed
    {
        if (empty($this->data)) throw new RuntimeException("Can't shift from an empty datastructure");
        return array_shift($this->data);
    }
    public function unshift(mixed $value): void
    {
        array_unshift($this->data, $value);
    }
    public function isEmpty(): bool { return empty($this->data); }
    public function count(): int { return count($this->data); }
    public function current(): mixed { return $this->data[$this->index] ?? null; }
    public function key(): mixed { return $this->index; }
    public function next(): void { $this->index++; }
    public function rewind(): void { $this->index = 0; }
    public function valid(): bool { return isset($this->data[$this->index]); }
    public function offsetExists(mixed $offset): bool { return isset($this->data[$offset]); }
    public function offsetGet(mixed $offset): mixed { return $this->data[$offset] ?? null; }
    public function offsetSet(mixed $offset, mixed $value): void
    {
        if ($offset === null) {
            $this->data[] = $value;
        } else {
            $this->data[$offset] = $value;
        }
    }
    public function offsetUnset(mixed $offset): void { unset($this->data[$offset]); }
    public function setIteratorMode(int $mode): int
    {
        $old = $this->flags;
        $this->flags = $mode;
        return $old;
    }
    public function getIteratorMode(): int { return $this->flags; }
}

class SplQueue extends SplDoublyLinkedList
{
    public function enqueue(mixed $value): void { $this->push($value); }
    public function dequeue(): mixed { return $this->shift(); }
}

class SplStack extends SplDoublyLinkedList
{
    public function __construct()
    {
        $this->setIteratorMode(self::IT_MODE_LIFO | self::IT_MODE_KEEP);
    }
}

class SplFixedArray implements Iterator, ArrayAccess, Countable
{
    protected array $data = [];
    protected int $size = 0;
    protected int $index = 0;

    public function __construct(int $size = 0)
    {
        $this->setSize($size);
    }

    public function count(): int { return $this->size; }
    public function getSize(): int { return $this->size; }
    public function setSize(int $size): bool
    {
        $this->size = $size;
        $this->data = array_pad(array_slice($this->data, 0, $size), $size, null);
        return true;
    }
    public function current(): mixed { return $this->data[$this->index] ?? null; }
    public function key(): mixed { return $this->index; }
    public function next(): void { $this->index++; }
    public function rewind(): void { $this->index = 0; }
    public function valid(): bool { return $this->index >= 0 && $this->index < $this->size; }
    public function offsetExists(mixed $offset): bool { return is_int($offset) && $offset >= 0 && $offset < $this->size && isset($this->data[$offset]); }
    public function offsetGet(mixed $offset): mixed
    {
        if (!is_int($offset) || $offset < 0 || $offset >= $this->size) {
            throw new RuntimeException("Index invalid or out of range");
        }
        return $this->data[$offset] ?? null;
    }
    public function offsetSet(mixed $offset, mixed $value): void
    {
        if (!is_int($offset) || $offset < 0 || $offset >= $this->size) {
            throw new RuntimeException("Index invalid or out of range");
        }
        $this->data[$offset] = $value;
    }
    public function offsetUnset(mixed $offset): void
    {
        if (is_int($offset) && $offset >= 0 && $offset < $this->size) {
            $this->data[$offset] = null;
        }
    }
    public function toArray(): array { return $this->data; }
    public static function fromArray(array $array, bool $preserveKeys = true): self
    {
        $obj = new self(count($array));
        $i = 0;
        foreach ($array as $k => $v) {
            $obj[$preserveKeys ? $k : $i++] = $v;
        }
        return $obj;
    }
}

class EmptyIterator implements Iterator
{
    public function current(): mixed { return null; }
    public function key(): mixed { return null; }
    public function next(): void {}
    public function rewind(): void {}
    public function valid(): bool { return false; }
}

class SplFileInfo implements Stringable
{
    private $pathname;

    public function __construct($filename)
    {
        $this->pathname = $filename;
    }

    public function getPath()
    {
        return dirname($this->pathname);
    }

    public function getFilename()
    {
        return basename($this->pathname);
    }

    public function getExtension()
    {
        return pathinfo($this->pathname, 4);
    }

    public function getBasename($suffix = "")
    {
        return basename($this->pathname, $suffix);
    }

    public function getPathname()
    {
        return $this->pathname;
    }

    public function getPerms()
    {
        return @fileperms($this->pathname);
    }

    public function getInode()
    {
        return @fileinode($this->pathname);
    }

    public function getSize()
    {
        return @filesize($this->pathname);
    }

    public function getOwner()
    {
        return @fileowner($this->pathname);
    }

    public function getGroup()
    {
        return @filegroup($this->pathname);
    }

    public function getATime()
    {
        return @fileatime($this->pathname);
    }

    public function getMTime()
    {
        return @filemtime($this->pathname);
    }

    public function getCTime()
    {
        return @filectime($this->pathname);
    }

    public function getType()
    {
        return @filetype($this->pathname);
    }

    public function isWritable()
    {
        return is_writable($this->pathname);
    }

    public function isReadable()
    {
        return is_readable($this->pathname);
    }

    public function isExecutable()
    {
        return is_executable($this->pathname);
    }

    public function isFile()
    {
        return is_file($this->pathname);
    }

    public function isDir()
    {
        return is_dir($this->pathname);
    }

    public function isLink()
    {
        return is_link($this->pathname);
    }

    public function getRealPath()
    {
        return realpath($this->pathname);
    }

    public function getFileInfo($class = null)
    {
        $cls = $class !== null ? $class : 'SplFileInfo';
        return new $cls($this->pathname);
    }

    public function getPathInfo($class = null)
    {
        $dir = $this->getPath();
        if ($dir === '') {
            return null;
        }
        $cls = $class !== null ? $class : 'SplFileInfo';
        return new $cls($dir);
    }

    public function getRelativePath(): string
    {
        return '';
    }

    public function getRelativePathname(): string
    {
        return $this->getFilename();
    }

    public function openFile($mode = 'r', $useIncludePath = false, $context = null): SplFileObject
    {
        return new SplFileObject($this->pathname, $mode, $useIncludePath, $context);
    }

    public function __toString(): string
    {
        return $this->pathname;
    }
}

class SplFileObject extends SplFileInfo implements SeekableIterator, RecursiveIterator
{
    private $handle = null;
    private $line = 0;
    private $currentLine = null;
    private $csvSeparator = ',';
    private $csvEnclosure = '"';
    private $csvEscape = '\\';

    public function __construct($filename, $mode = 'r', $useIncludePath = false, $context = null)
    {
        parent::__construct($filename);
        if ($context !== null) {
            $this->handle = @fopen($filename, $mode, $useIncludePath, $context);
        } else {
            $this->handle = @fopen($filename, $mode, $useIncludePath);
        }
        $this->line = 0;
        $this->currentLine = null;
    }

    public function __destruct()
    {
        if (is_resource($this->handle)) {
            @fclose($this->handle);
            $this->handle = null;
        }
    }

    public function current(): mixed
    {
        if ($this->currentLine === null && $this->handle && !feof($this->handle)) {
            $line = fgets($this->handle);
            $this->currentLine = ($line !== false) ? $line : false;
        }
        return $this->currentLine;
    }

    public function key(): mixed
    {
        return $this->line;
    }

    public function next(): void
    {
        if ($this->handle && !feof($this->handle)) {
            $line = fgets($this->handle);
            $this->currentLine = ($line !== false) ? $line : false;
        } else {
            $this->currentLine = false;
        }
        $this->line++;
    }

    public function rewind(): void
    {
        $this->line = 0;
        $this->currentLine = null;
        if ($this->handle) {
            @rewind($this->handle);
        }
    }

    public function valid(): bool
    {
        if ($this->currentLine !== null) {
            return $this->currentLine !== false;
        }
        return $this->handle ? !feof($this->handle) : false;
    }

    public function seek($line): void
    {
        $this->rewind();
        while ($this->line < $line && $this->valid()) {
            $this->next();
        }
    }

    public function eof(): bool
    {
        return $this->handle ? feof($this->handle) : true;
    }

    public function fgets(): string|false
    {
        if (!$this->handle || feof($this->handle)) {
            $this->currentLine = false;
            return false;
        }
        $line = fgets($this->handle);
        $this->currentLine = ($line !== false) ? $line : false;
        if ($line !== false) {
            $this->line++;
        }
        return $line;
    }

    public function fgetcsv($separator = ",", $enclosure = "\"", $escape = "\\"): array|false
    {
        $this->currentLine = null;
        return $this->handle ? fgetcsv($this->handle, 0, $separator, $enclosure, $escape) : false;
    }

    public function fwrite($string, $length = null): int|false
    {
        if (!$this->handle) return false;
        $this->currentLine = null;
        return $length !== null ? fwrite($this->handle, $string, $length) : fwrite($this->handle, $string);
    }

    public function fflush(): bool
    {
        return $this->handle ? fflush($this->handle) : false;
    }

    public function ftell(): int|false
    {
        return $this->handle ? ftell($this->handle) : false;
    }

    public function fseek($offset, $whence = 0): int
    {
        $this->currentLine = null;
        return $this->handle ? fseek($this->handle, $offset, $whence) : -1;
    }

    public function fstat(): array|false
    {
        return $this->handle ? fstat($this->handle) : false;
    }

    public function ftruncate($size): bool
    {
        $this->currentLine = null;
        return $this->handle ? ftruncate($this->handle, $size) : false;
    }

    public function flock($operation, &$wouldBlock = null): bool
    {
        return $this->handle ? flock($this->handle, $operation, $wouldBlock) : false;
    }

    public function fpassthru(): int
    {
        $this->currentLine = null;
        return $this->handle ? fpassthru($this->handle) : 0;
    }

    public function fread($length): string|false
    {
        $this->currentLine = null;
        return $this->handle ? fread($this->handle, $length) : false;
    }

    public function setCsvControl($separator = ",", $enclosure = "\"", $escape = "\\"): void
    {
        $this->csvSeparator = $separator;
        $this->csvEnclosure = $enclosure;
        $this->csvEscape = $escape;
    }

    public function getCsvControl(): array
    {
        return [$this->csvSeparator, $this->csvEnclosure, $this->csvEscape];
    }

    public function hasChildren(): bool { return false; }
    public function getChildren(): ?RecursiveIterator { return null; }
}



class DirectoryIterator extends SplFileInfo implements SeekableIterator
{
    protected $entries = [];
    protected $position = 0;
    protected $dirPath;

    public function __construct($directory)
    {
        parent::__construct($directory);
        $this->dirPath = rtrim($directory, DIRECTORY_SEPARATOR);
        $this->readDir();
    }

    protected function readDir(): void
    {
        $this->entries = [];
        $this->position = 0;
        if (is_dir($this->dirPath)) {
            $items = scandir($this->dirPath);
            if ($items !== false) {
                $this->entries = $items;
            }
        }
    }

    public function current(): mixed
    {
        return $this;
    }

    public function key(): mixed
    {
        return $this->position;
    }

    public function next(): void
    {
        $this->position = $this->position + 1;
    }

    public function rewind(): void
    {
        $this->position = 0;
    }

    public function valid(): bool
    {
        return $this->position >= 0 && $this->position < count($this->entries);
    }

    public function seek($offset): void
    {
        $this->position = $offset;
    }

    public function isDot(): bool
    {
        $fn = $this->getFilename();
        return $fn === '.' || $fn === '..';
    }

    public function getFilename()
    {
        return isset($this->entries[$this->position]) ? $this->entries[$this->position] : '';
    }

    public function getPathname()
    {
        $fn = $this->getFilename();
        return $this->dirPath . DIRECTORY_SEPARATOR . $fn;
    }

    public function getPath()
    {
        return $this->dirPath;
    }

    public function isFile()
    {
        return is_file($this->getPathname());
    }

    public function isDir()
    {
        return is_dir($this->getPathname());
    }

    public function isLink()
    {
        return is_link($this->getPathname());
    }

    public function getRealPath()
    {
        return realpath($this->getPathname());
    }

    public function getSize()
    {
        return @filesize($this->getPathname());
    }

    public function getMTime()
    {
        return @filemtime($this->getPathname());
    }

    public function __toString(): string
    {
        return $this->getFilename();
    }
}

class FilesystemIterator extends DirectoryIterator
{
    public const CURRENT_AS_PATHNAME = 32;
    public const CURRENT_AS_FILEINFO = 0;
    public const CURRENT_AS_SELF = 16;
    public const CURRENT_MODE_MASK = 240;
    public const KEY_AS_PATHNAME = 0;
    public const KEY_AS_FILENAME = 256;
    public const FOLLOW_SYMLINKS = 512;
    public const KEY_MODE_MASK = 3840;
    public const NEW_CURRENT_AND_KEY = 256;
    public const SKIP_DOTS = 4096;
    public const UNIX_PATHS = 8192;

    protected $flags;

    public function __construct($directory, $flags = 4096)
    {
        $this->flags = $flags;
        parent::__construct($directory);
    }

    protected function readDir(): void
    {
        parent::readDir();
        if (($this->flags & self::SKIP_DOTS) !== 0) {
            $filtered = [];
            foreach ($this->entries as $e) {
                if ($e !== '.' && $e !== '..') {
                    $filtered[] = $e;
                }
            }
            $this->entries = $filtered;
        }
    }

    public function getFlags(): int
    {
        return $this->flags;
    }

    public function setFlags($flags): void
    {
        $this->flags = $flags;
    }

    public function current(): mixed
    {
        $mode = $this->flags & self::CURRENT_MODE_MASK;
        if ($mode === self::CURRENT_AS_PATHNAME) {
            return $this->getPathname();
        }
        if ($mode === self::CURRENT_AS_SELF) {
            return $this;
        }
        return new SplFileInfo($this->getPathname());
    }

    public function key(): mixed
    {
        $mode = $this->flags & self::KEY_MODE_MASK;
        if ($mode === self::KEY_AS_FILENAME) {
            return $this->getFilename();
        }
        return $this->getPathname();
    }
}

class RecursiveDirectoryIterator extends FilesystemIterator implements RecursiveIterator
{
    private $subPath = '';

    public function __construct($directory, $flags = 4096)
    {
        parent::__construct($directory, $flags);
    }

    public function hasChildren($allowLinks = false): bool
    {
        if ($this->isDot()) {
            return false;
        }
        if ($this->isLink() && !$allowLinks) {
            return false;
        }
        return $this->isDir();
    }

    public function getChildren(): RecursiveDirectoryIterator
    {
        $sub = new RecursiveDirectoryIterator($this->getPathname(), $this->flags);
        $sub->subPath = ($this->subPath !== '' ? $this->subPath . DIRECTORY_SEPARATOR : '') . $this->getFilename();
        return $sub;
    }

    public function getSubPath(): string
    {
        return $this->subPath ?? '';
    }

    public function getSubPathname(): string
    {
        $sp = $this->subPath ?? '';
        return ($sp !== '' ? $sp . DIRECTORY_SEPARATOR : '') . $this->getFilename();
    }
}


class RecursiveIteratorIterator implements OuterIterator
{
    public const LEAVES_ONLY = 0;
    public const SELF_FIRST = 1;
    public const CHILD_FIRST = 2;
    public const CATCH_GET_CHILD = 16;

    protected $iterators = [];
    protected $mode;
    protected $flags;

    public function __construct($iterator, $mode = 0, $flags = 0)
    {
        $this->mode = $mode;
        $this->flags = $flags;
        if ($iterator instanceof Iterator) {
            $this->iterators[] = $iterator;
        } elseif ($iterator instanceof IteratorAggregate) {
            $this->iterators[] = $iterator->getIterator();
        }
    }

    public function getInnerIterator(): Iterator
    {
        return end($this->iterators);
    }

    public function current(): mixed
    {
        $it = end($this->iterators);
        return $it ? $it->current() : null;
    }

    public function key(): mixed
    {
        $it = end($this->iterators);
        return $it ? $it->key() : null;
    }

    public function valid(): bool
    {
        $it = end($this->iterators);
        return $it ? $it->valid() : false;
    }

    public function rewind(): void
    {
        while (count($this->iterators) > 1) {
            array_pop($this->iterators);
        }
        if (!empty($this->iterators)) {
            $this->iterators[0]->rewind();
            $this->advanceToNextValid();
        }
    }

    public function next(): void
    {
        $it = end($this->iterators);
        if ($it) {
            if ($it instanceof RecursiveIterator && $it->hasChildren() && $this->mode !== self::CHILD_FIRST) {
                $child = $it->getChildren();
                $child->rewind();
                $this->iterators[] = $child;
            } else {
                $it->next();
            }
            $this->advanceToNextValid();
        }
    }

    protected function advanceToNextValid(): void
    {
        while (!empty($this->iterators)) {
            $it = end($this->iterators);
            if ($it->valid()) {
                if ($this->mode === self::LEAVES_ONLY && $it instanceof RecursiveIterator && $it->hasChildren()) {
                    $child = $it->getChildren();
                    $child->rewind();
                    $this->iterators[] = $child;
                    continue;
                }
                return;
            }
            array_pop($this->iterators);
            if (!empty($this->iterators)) {
                end($this->iterators)->next();
            }
        }
    }

    public function getDepth(): int
    {
        return count($this->iterators) - 1;
    }

    public function getSubIterator($level = null)
    {
        $idx = $level !== null ? $level : (count($this->iterators) - 1);
        return isset($this->iterators[$idx]) ? $this->iterators[$idx] : null;
    }
}

class IteratorIterator implements OuterIterator
{
    protected $iterator;

    public function __construct(Traversable $iterator, ?string $class = null)
    {
        if ($iterator instanceof Iterator) {
            $this->iterator = $iterator;
        } elseif ($iterator instanceof IteratorAggregate) {
            $this->iterator = $iterator->getIterator();
        } else {
            $this->iterator = new ArrayIterator([]);
        }
    }

    public function getInnerIterator(): ?Iterator
    {
        return $this->iterator;
    }

    public function current(): mixed
    {
        return $this->iterator ? $this->iterator->current() : null;
    }

    public function key(): mixed
    {
        return $this->iterator ? $this->iterator->key() : null;
    }

    public function next(): void
    {
        if ($this->iterator) {
            $this->iterator->next();
        }
    }

    public function rewind(): void
    {
        if ($this->iterator) {
            $this->iterator->rewind();
        }
    }

    public function valid(): bool
    {
        return $this->iterator ? $this->iterator->valid() : false;
    }

    public function __call(string $func, array $params): mixed
    {
        if ($this->iterator) {
            return $this->iterator->$func(...$params);
        }
        throw new BadMethodCallException("Call to undefined method " . get_class($this) . "::$func()");
    }
}

abstract class FilterIterator extends IteratorIterator
{
    public function __construct(Iterator $iterator)
    {
        parent::__construct($iterator);
    }

    abstract public function accept(): bool;

    public function rewind(): void
    {
        parent::rewind();
        $this->advanceToNextAccepted();
    }

    public function next(): void
    {
        parent::next();
        $this->advanceToNextAccepted();
    }

    protected function advanceToNextAccepted(): void
    {
        while ($this->valid()) {
            if ($this->accept()) {
                return;
            }
            parent::next();
        }
    }
}

class CallbackFilterIterator extends FilterIterator
{
    private $callback;

    public function __construct(Iterator $iterator, $callback)
    {
        parent::__construct($iterator);
        $this->callback = $callback;
    }

    public function accept(): bool
    {
        $cb = $this->callback;
        return (bool)$cb($this->current(), $this->key(), $this->getInnerIterator());
    }
}

abstract class RecursiveFilterIterator extends FilterIterator implements RecursiveIterator
{
    public function __construct(RecursiveIterator $iterator)
    {
        parent::__construct($iterator);
    }

    public function hasChildren(): bool
    {
        $inner = $this->getInnerIterator();
        return $inner instanceof RecursiveIterator && $inner->hasChildren();
    }

    public function getChildren(): ?RecursiveFilterIterator
    {
        $inner = $this->getInnerIterator();
        if ($inner instanceof RecursiveIterator) {
            $children = $inner->getChildren();
            return $children !== null ? new static($children) : null;
        }
        return null;
    }
}

class RecursiveCallbackFilterIterator extends RecursiveFilterIterator
{
    private $callback;

    public function __construct(RecursiveIterator $iterator, $callback)
    {
        parent::__construct($iterator);
        $this->callback = $callback;
    }

    public function accept(): bool
    {
        $cb = $this->callback;
        return (bool)$cb($this->current(), $this->key(), $this->getInnerIterator());
    }

    public function getChildren(): ?RecursiveCallbackFilterIterator
    {
        $inner = $this->getInnerIterator();
        if ($inner instanceof RecursiveIterator) {
            $children = $inner->getChildren();
            return $children !== null ? new static($children, $this->callback) : null;
        }
        return null;
    }
}

class ParentIterator extends FilterIterator implements RecursiveIterator
{
    public function __construct(RecursiveIterator $iterator)
    {
        parent::__construct($iterator);
    }

    public function accept(): bool
    {
        $inner = $this->getInnerIterator();
        return $inner instanceof RecursiveIterator && $inner->hasChildren();
    }

    public function hasChildren($allowLinks = false): bool
    {
        $inner = $this->getInnerIterator();
        return $inner instanceof RecursiveIterator && $inner->hasChildren($allowLinks);
    }

    public function getChildren(): ?RecursiveIterator
    {
        $inner = $this->getInnerIterator();
        return $inner instanceof RecursiveIterator ? new self($inner->getChildren()) : null;
    }
}

class AppendIterator implements OuterIterator
{
    private $iterators;
    private $pos = 0;

    public function __construct()
    {
        $this->iterators = new ArrayIterator([]);
    }

    public function append(Iterator $iterator): void
    {
        $this->iterators->append($iterator);
    }

    public function getInnerIterator(): ?Iterator
    {
        return isset($this->iterators[$this->pos]) ? $this->iterators[$this->pos] : null;
    }

    public function getArrayIterator(): ArrayIterator
    {
        return $this->iterators;
    }

    public function rewind(): void
    {
        $this->pos = 0;
        $this->iterators->rewind();
        if ($this->iterators->valid()) {
            $this->iterators->current()->rewind();
            $this->advanceToNextValid();
        }
    }

    public function valid(): bool
    {
        return $this->iterators->valid() && $this->iterators->current()->valid();
    }

    public function current(): mixed
    {
        return $this->valid() ? $this->iterators->current()->current() : null;
    }

    public function key(): mixed
    {
        return $this->valid() ? $this->iterators->current()->key() : null;
    }

    public function next(): void
    {
        if ($this->valid()) {
            $this->iterators->current()->next();
            $this->advanceToNextValid();
        }
    }

    private function advanceToNextValid(): void
    {
        while ($this->iterators->valid() && !$this->iterators->current()->valid()) {
            $this->pos++;
            $this->iterators->next();
            if ($this->iterators->valid()) {
                $this->iterators->current()->rewind();
            }
        }
    }
}

class WeakMap implements ArrayAccess, Countable, IteratorAggregate
{
    private $map = [];

    public function offsetExists(mixed $object): bool
    {
        if (!is_object($object)) return false;
        $id = spl_object_id($object);
        return isset($this->map[$id]);
    }

    public function offsetGet(mixed $object): mixed
    {
        if (!is_object($object)) return null;
        $id = spl_object_id($object);
        if (isset($this->map[$id])) {
            return $this->map[$id]['value'];
        }
        return null;
    }

    public function offsetSet(mixed $object, mixed $value): void
    {
        if (!is_object($object)) {
            if (is_object($value)) {
                $tmp = $object;
                $object = $value;
                $value = $tmp;
            } else {
                throw new TypeError("Cannot use non-object as key in WeakMap (got " . gettype($object) . ": " . var_export($object, true) . ")");
            }
        }
        $id = spl_object_id($object);
        $this->map[$id] = [
            'key' => $object,
            'value' => $value,
        ];
    }

    public function offsetUnset(mixed $object): void
    {
        if (!is_object($object)) return;
        $id = spl_object_id($object);
        unset($this->map[$id]);
    }

    public function count(): int
    {
        return count($this->map);
    }

    public function getIterator(): Traversable
    {
        $entries = [];
        foreach ($this->map as $entry) {
            $entries[] = $entry['value'];
        }
        return new ArrayIterator($entries);
    }
}

class DOMNode
{
    public $nodeName = null;
    public $nodeValue = null;
    public $nodeType = 1;
    public $parentNode = null;
    public $childNodes = null;
    public $firstChild = null;
    public $lastChild = null;
    public $previousSibling = null;
    public $nextSibling = null;
    public $attributes = null;
    public $ownerDocument = null;

    public function __construct()
    {
        $this->childNodes = new DOMNodeList();
        $this->attributes = new DOMNamedNodeMap();
    }

    public function appendChild(DOMNode $node): DOMNode
    {
        $node->parentNode = $this;
        $node->ownerDocument = $this->ownerDocument ?? ($this instanceof DOMDocument ? $this : null);
        
        $currentChildren = $this->childNodes->getNodes();
        $count = count($currentChildren);
        if ($count > 0) {
            $prev = $currentChildren[$count - 1];
            $prev->nextSibling = $node;
            $node->previousSibling = $prev;
        } else {
            $this->firstChild = $node;
        }
        $node->nextSibling = null;
        $this->lastChild = $node;

        $this->childNodes->addNode($node);
        return $node;
    }

    public function removeChild(DOMNode $child): DOMNode
    {
        $nodes = $this->childNodes->getNodes();
        $newNodes = [];
        foreach ($nodes as $n) {
            if ($n === $child) {
                if ($n->previousSibling) {
                    $n->previousSibling->nextSibling = $n->nextSibling;
                }
                if ($n->nextSibling) {
                    $n->nextSibling->previousSibling = $n->previousSibling;
                }
                if ($this->firstChild === $n) {
                    $this->firstChild = $n->nextSibling;
                }
                if ($this->lastChild === $n) {
                    $this->lastChild = $n->previousSibling;
                }
                $n->parentNode = null;
                $n->previousSibling = null;
                $n->nextSibling = null;
            } else {
                $newNodes[] = $n;
            }
        }
        $this->childNodes->setNodes($newNodes);
        return $child;
    }

    public function insertBefore(DOMNode $node, ?DOMNode $child = null): DOMNode
    {
        if ($child === null) {
            return $this->appendChild($node);
        }
        $node->parentNode = $this;
        $node->ownerDocument = $this->ownerDocument ?? ($this instanceof DOMDocument ? $this : null);

        $nodes = $this->childNodes->getNodes();
        $newNodes = [];
        foreach ($nodes as $n) {
            if ($n === $child) {
                $node->previousSibling = $n->previousSibling;
                $node->nextSibling = $n;
                if ($n->previousSibling) {
                    $n->previousSibling->nextSibling = $node;
                } else {
                    $this->firstChild = $node;
                }
                $n->previousSibling = $node;
                $newNodes[] = $node;
            }
            $newNodes[] = $n;
        }
        $this->childNodes->setNodes($newNodes);
        return $node;
    }

    public function replaceChild(DOMNode $node, DOMNode $child): DOMNode
    {
        $this->insertBefore($node, $child);
        return $this->removeChild($child);
    }

    public function hasChildNodes(): bool
    {
        return $this->childNodes !== null && $this->childNodes->count() > 0;
    }

    public function hasAttributes(): bool
    {
        return $this->attributes !== null && $this->attributes->count() > 0;
    }

    public function cloneNode(bool $deep = false): DOMNode
    {
        $cls = get_class($this);
        $clone = new $cls();
        $clone->nodeName = $this->nodeName;
        $clone->nodeValue = $this->nodeValue;
        $clone->nodeType = $this->nodeType;
        $clone->textContent = $this->textContent;
        if ($this->attributes) {
            foreach ($this->attributes->getNodes() as $attr) {
                $clone->attributes->addNode(new DOMAttr($attr->name, $attr->value));
            }
        }
        if ($deep && $this->childNodes) {
            foreach ($this->childNodes->getNodes() as $child) {
                $clone->appendChild($child->cloneNode(true));
            }
        }
        return $clone;
    }

    public function __get(string $name)
    {
        if ($name === 'textContent') {
            if ($this->nodeType === 3 || $this->nodeType === 4 || $this->nodeType === 8 || $this->nodeType === 2) {
                return (string)($this->nodeValue ?? '');
            }
            $text = '';
            if ($this->childNodes) {
                foreach ($this->childNodes->getNodes() as $child) {
                    $text .= (string)$child->textContent;
                }
            }
            return $text;
        }
        return null;
    }
}

class DOMElement extends DOMNode
{
    public $tagName = '';

    public function __construct(string $name = '', ?string $value = null, string $uri = '')
    {
        parent::__construct();
        $this->nodeType = 1;
        $this->nodeName = $name;
        $this->tagName = $name;
        $this->nodeValue = $value;
        if ($value !== null) {
            $this->textContent = $value;
        }
    }

    public function getAttribute(string $qualifiedName): string
    {
        $attr = $this->attributes ? $this->attributes->getNamedItem($qualifiedName) : null;
        return $attr ? $attr->value : '';
    }

    public function setAttribute(string $qualifiedName, string $value): bool
    {
        if ($this->attributes === null) {
            $this->attributes = new DOMNamedNodeMap();
        }
        $attr = new DOMAttr($qualifiedName, $value);
        $attr->ownerDocument = $this->ownerDocument;
        $attr->parentNode = $this;
        $this->attributes->setNamedItem($attr);
        return true;
    }

    public function removeAttribute(string $qualifiedName): bool
    {
        if ($this->attributes) {
            return $this->attributes->removeNamedItem($qualifiedName);
        }
        return false;
    }

    public function hasAttribute(string $qualifiedName): bool
    {
        return $this->attributes !== null && $this->attributes->getNamedItem($qualifiedName) !== null;
    }

    public function getElementsByTagName(string $qualifiedName): DOMNodeList
    {
        $result = [];
        $lower = strtolower($qualifiedName);
        $this->_collectElementsByTagName($this, $lower, $result);
        return new DOMNodeList($result);
    }

    private function _collectElementsByTagName(DOMNode $parent, string $name, array &$result): void
    {
        if ($parent->childNodes) {
            foreach ($parent->childNodes->getNodes() as $child) {
                if ($child instanceof DOMElement) {
                    if ($name === '*' || strtolower($child->tagName) === $name) {
                        $result[] = $child;
                    }
                    $this->_collectElementsByTagName($child, $name, $result);
                }
            }
        }
    }
}

class DOMText extends DOMNode
{
    public $wholeText = '';

    public function __construct(string $data = '')
    {
        parent::__construct();
        $this->nodeType = 3;
        $this->nodeName = '#text';
        $this->nodeValue = $data;
        $this->wholeText = $data;
        $this->textContent = $data;
    }
}

class DOMComment extends DOMNode
{
    public function __construct(string $data = '')
    {
        parent::__construct();
        $this->nodeType = 8;
        $this->nodeName = '#comment';
        $this->nodeValue = $data;
        $this->textContent = $data;
    }
}

class DOMAttr extends DOMNode
{
    public $name = '';
    public $value = '';

    public function __construct(string $name = '', string $value = '')
    {
        parent::__construct();
        $this->nodeType = 2;
        $this->nodeName = $name;
        $this->name = $name;
        $this->value = $value;
        $this->nodeValue = $value;
        $this->textContent = $value;
    }
}

class DOMNodeList implements Countable, IteratorAggregate
{
    private $nodes = [];
    public $length = 0;

    public function __construct(array $nodes = [])
    {
        $this->setNodes($nodes);
    }

    public function addNode(DOMNode $node): void
    {
        $this->nodes[] = $node;
        $this->length = count($this->nodes);
    }

    public function setNodes(array $nodes): void
    {
        $this->nodes = array_values($nodes);
        $this->length = count($this->nodes);
    }

    public function getNodes(): array
    {
        return $this->nodes;
    }

    public function item(int $index): ?DOMNode
    {
        return $this->nodes[$index] ?? null;
    }

    public function count(): int
    {
        return $this->length;
    }

    public function getIterator(): Traversable
    {
        return new ArrayIterator($this->nodes);
    }
}

class DOMNamedNodeMap implements Countable, IteratorAggregate
{
    private $nodes = [];
    public $length = 0;

    public function __construct(array $nodes = [])
    {
        $this->setNodes($nodes);
    }

    public function getNamedItem(string $qualifiedName): ?DOMNode
    {
        $lower = strtolower($qualifiedName);
        foreach ($this->nodes as $node) {
            if (strtolower($node->nodeName) === $lower) {
                return $node;
            }
        }
        return null;
    }

    public function setNamedItem(DOMNode $node): void
    {
        $lower = strtolower($node->nodeName);
        foreach ($this->nodes as $i => $n) {
            if (strtolower($n->nodeName) === $lower) {
                $this->nodes[$i] = $node;
                return;
            }
        }
        $this->nodes[] = $node;
        $this->length = count($this->nodes);
    }

    public function removeNamedItem(string $qualifiedName): bool
    {
        $lower = strtolower($qualifiedName);
        foreach ($this->nodes as $i => $n) {
            if (strtolower($n->nodeName) === $lower) {
                array_splice($this->nodes, $i, 1);
                $this->length = count($this->nodes);
                return true;
            }
        }
        return false;
    }

    public function setNodes(array $nodes): void
    {
        $this->nodes = array_values($nodes);
        $this->length = count($this->nodes);
    }

    public function getNodes(): array
    {
        return $this->nodes;
    }

    public function item(int $index): ?DOMNode
    {
        return $this->nodes[$index] ?? null;
    }

    public function count(): int
    {
        return $this->length;
    }

    public function getIterator(): Traversable
    {
        return new ArrayIterator($this->nodes);
    }
}

class DOMDocument extends DOMNode
{
    public $documentElement = null;
    public $body = null;
    public $version = '1.0';
    public $encoding = 'UTF-8';

    public function __construct(string $version = '1.0', string $encoding = '')
    {
        parent::__construct();
        $this->nodeType = 9;
        $this->nodeName = '#document';
        $this->version = $version;
        $this->encoding = $encoding ?: 'UTF-8';
        $this->ownerDocument = $this;
    }

    public function createElement(string $localName, string $value = ''): DOMElement|false
    {
        $el = new DOMElement($localName, $value !== '' ? $value : null);
        $el->ownerDocument = $this;
        return $el;
    }

    public function createTextNode(string $data): DOMText
    {
        $t = new DOMText($data);
        $t->ownerDocument = $this;
        return $t;
    }

    public function createComment(string $data): DOMComment
    {
        $c = new DOMComment($data);
        $c->ownerDocument = $this;
        return $c;
    }

    public function createAttribute(string $localName): DOMAttr|false
    {
        $a = new DOMAttr($localName);
        $a->ownerDocument = $this;
        return $a;
    }

    public function getElementsByTagName(string $qualifiedName): DOMNodeList
    {
        $result = [];
        $lower = strtolower($qualifiedName);
        $this->_collectElementsByTagName($this, $lower, $result);
        return new DOMNodeList($result);
    }

    private function _collectElementsByTagName(DOMNode $parent, string $name, array &$result): void
    {
        if ($parent->childNodes) {
            foreach ($parent->childNodes->getNodes() as $child) {
                if ($child instanceof DOMElement) {
                    if ($name === '*' || strtolower($child->tagName) === $name) {
                        $result[] = $child;
                    }
                    $this->_collectElementsByTagName($child, $name, $result);
                }
            }
        }
    }

    public function getElementById(string $elementId): ?DOMElement
    {
        return $this->_findId($this, $elementId);
    }

    private function _findId(DOMNode $parent, string $id): ?DOMElement
    {
        if ($parent->childNodes) {
            foreach ($parent->childNodes->getNodes() as $child) {
                if ($child instanceof DOMElement) {
                    if ($child->getAttribute('id') === $id) {
                        return $child;
                    }
                    $found = $this->_findId($child, $id);
                    if ($found !== null) {
                        return $found;
                    }
                }
            }
        }
        return null;
    }

    public function loadHTML(string $source, int $options = 0): bool
    {
        $this->childNodes->setNodes([]);
        $this->firstChild = null;
        $this->lastChild = null;
        $this->documentElement = null;
        $this->body = null;

        $p = new _DOMHtmlParser($this);
        return $p->parse($source);
    }

    public function loadXML(string $source, int $options = 0): bool
    {
        $this->childNodes->setNodes([]);
        $this->firstChild = null;
        $this->lastChild = null;
        $this->documentElement = null;
        $this->body = null;

        $p = new _DOMHtmlParser($this, true);
        return $p->parse($source);
    }

    public function saveHTML(?DOMNode $node = null): string|false
    {
        $target = $node ?? $this->documentElement ?? $this;
        return $this->_renderNode($target, false);
    }

    public function saveXML(?DOMNode $node = null, int $options = 0): string|false
    {
        $target = $node ?? $this->documentElement ?? $this;
        return $this->_renderNode($target, true);
    }

    public function schemaValidateSource(string $source, int $flags = 0): bool
    {
        return true;
    }

    public function schemaValidate(string $filename, int $flags = 0): bool
    {
        return true;
    }

    public function relaxNGValidateSource(string $source): bool
    {
        return true;
    }

    public function relaxNGValidate(string $filename): bool
    {
        return true;
    }

    public function validate(): bool
    {
        return true;
    }

    public function importNode(DOMNode $node, bool $deep = false): DOMNode|false
    {
        return $node->cloneNode($deep);
    }

    private function _renderNode(DOMNode $node, bool $xml): string
    {
        if ($node instanceof DOMText) {
            return $xml ? htmlspecialchars($node->nodeValue ?? '', ENT_XML1) : ($node->nodeValue ?? '');
        }
        if ($node instanceof DOMComment) {
            return '<!--' . ($node->nodeValue ?? '') . '-->';
        }
        if ($node instanceof DOMElement) {
            $out = '<' . $node->tagName;
            if ($node->attributes) {
                foreach ($node->attributes->getNodes() as $attr) {
                    $val = htmlspecialchars($attr->value ?? '', ENT_QUOTES);
                    $out .= ' ' . $attr->name . '="' . $val . '"';
                }
            }
            $children = $node->childNodes ? $node->childNodes->getNodes() : [];
            $voids = ['area'=>1,'base'=>1,'br'=>1,'col'=>1,'embed'=>1,'hr'=>1,'img'=>1,'input'=>1,'link'=>1,'meta'=>1,'param'=>1,'source'=>1,'track'=>1,'wbr'=>1];
            if (empty($children)) {
                if ($xml) {
                    return $out . ' />';
                }
                if (isset($voids[strtolower($node->tagName)])) {
                    return $out . '>';
                }
                return $out . '></' . $node->tagName . '>';
            }
            $out .= '>';
            foreach ($children as $child) {
                $out .= $this->_renderNode($child, $xml);
            }
            $out .= '</' . $node->tagName . '>';
            return $out;
        }
        if ($node instanceof DOMDocument) {
            $out = '';
            if ($node->childNodes) {
                foreach ($node->childNodes->getNodes() as $child) {
                    $out .= $this->_renderNode($child, $xml);
                }
            }
            return $out;
        }
        return '';
    }
}

class _DOMHtmlParser
{
    private $doc;
    private $isXml;

    public function __construct(DOMDocument $doc, bool $isXml = false)
    {
        $this->doc = $doc;
        $this->isXml = $isXml;
    }

    public function parse(string $html): bool
    {
        $len = strlen($html);
        $i = 0;
        $stack = [$this->doc];
        $voids = $this->isXml ? [] : ['area'=>1,'base'=>1,'br'=>1,'col'=>1,'embed'=>1,'hr'=>1,'img'=>1,'input'=>1,'link'=>1,'meta'=>1,'param'=>1,'source'=>1,'track'=>1,'wbr'=>1];

        while ($i < $len) {
            $lt = strpos($html, '<', $i);
            if ($lt === false) {
                $text = substr($html, $i);
                if ($text !== '' && (!$this->isXml || $this->doc->preserveWhiteSpace || trim($text) !== '')) {
                    $current = $stack[count($stack) - 1];
                    $node = $this->doc->createTextNode(html_entity_decode($text, ENT_QUOTES | ENT_HTML5));
                    $current->appendChild($node);
                }
                break;
            }

            if ($lt > $i) {
                $text = substr($html, $i, $lt - $i);
                if ($text !== '' && (!$this->isXml || $this->doc->preserveWhiteSpace || trim($text) !== '')) {
                    $current = $stack[count($stack) - 1];
                    $node = $this->doc->createTextNode(html_entity_decode($text, ENT_QUOTES | ENT_HTML5));
                    $current->appendChild($node);
                }
                $i = $lt;
            }

            // Check comment: <!-- ... -->
            if (substr($html, $i, 4) === '<!--') {
                $end = strpos($html, '-->', $i + 4);
                if ($end === false) {
                    $commentText = substr($html, $i + 4);
                    $i = $len;
                } else {
                    $commentText = substr($html, $i + 4, $end - ($i + 4));
                    $i = $end + 3;
                }
                $current = $stack[count($stack) - 1];
                $node = $this->doc->createComment($commentText);
                $current->appendChild($node);
                continue;
            }

         
            if (substr($html, $i, 2) === '<?' || strtolower(substr($html, $i, 9)) === '<!doctype') {
                $end = strpos($html, '>', $i);
                if ($end === false) {
                    $i = $len;
                } else {
                    $i = $end + 1;
                }
                continue;
            }

            // Check closing tag: </tag>
            if (substr($html, $i, 2) === '</') {
                $end = strpos($html, '>', $i + 2);
                if ($end === false) {
                    $tagName = trim(substr($html, $i + 2));
                    $i = $len;
                } else {
                    $tagName = trim(substr($html, $i + 2, $end - ($i + 2)));
                    $i = $end + 1;
                }
                $lower = strtolower($tagName);
                // Pop stack until matching tag
                for ($s = count($stack) - 1; $s > 0; $s--) {
                    if ($stack[$s] instanceof DOMElement && strtolower($stack[$s]->tagName) === $lower) {
                        array_splice($stack, $s);
                        break;
                    }
                }
                continue;
            }

            // Opening tag: <tag attr...>
            $gt = strpos($html, '>', $i + 1);
            if ($gt === false) {
                $tagContent = substr($html, $i + 1);
                $i = $len;
            } else {
                $tagContent = substr($html, $i + 1, $gt - ($i + 1));
                $i = $gt + 1;
            }

            $isSelfClosing = false;
            if (str_ends_with(rtrim($tagContent), '/')) {
                $isSelfClosing = true;
                $tagContent = rtrim(substr(rtrim($tagContent), 0, -1));
            }

            // Extract tag name
            $tagContent = trim($tagContent);
            if ($tagContent === '') {
                continue;
            }
            $spacePos = strpos($tagContent, ' ');
            $tabPos = strpos($tagContent, "\t");
            $nlPos = strpos($tagContent, "\n");
            $firstWs = false;
            foreach ([$spacePos, $tabPos, $nlPos] as $p) {
                if ($p !== false && ($firstWs === false || $p < $firstWs)) {
                    $firstWs = $p;
                }
            }

            if ($firstWs === false) {
                $tagName = $tagContent;
                $attrString = '';
            } else {
                $tagName = substr($tagContent, 0, $firstWs);
                $attrString = substr($tagContent, $firstWs + 1);
            }

            $el = $this->doc->createElement($tagName);
            if (strtolower($tagName) === 'html' && $this->doc->documentElement === null) {
                $this->doc->documentElement = $el;
            }
            if (strtolower($tagName) === 'body' && $this->doc->body === null) {
                $this->doc->body = $el;
            }

            // Parse attributes
            $this->_parseAttributes($el, $attrString);

            $current = $stack[count($stack) - 1];
            $current->appendChild($el);

            $lower = strtolower($tagName);
            if (!$isSelfClosing && !isset($voids[$lower])) {
                $stack[] = $el;
            }
        }

        if ($this->doc->documentElement === null) {
            foreach ($this->doc->childNodes->getNodes() as $c) {
                if ($c instanceof DOMElement) {
                    $this->doc->documentElement = $c;
                    break;
                }
            }
        }

        return true;
    }

    private function _parseAttributes(DOMElement $el, string $attrString): void
    {
        if (trim($attrString) === '') {
            return;
        }
        $matches = [];
        if (preg_match_all('/([a-zA-Z0-9_:-]+)(?:\s*=\s*(?:"([^"]*)"|\'([^\']*)\'|([^\s>]+)))?/', $attrString, $matches, PREG_SET_ORDER)) {
            foreach ($matches as $m) {
                $name = $m[1];
                $val = $name;
                if (isset($m[2]) && $m[2] !== '') {
                    $val = $m[2];
                } elseif (isset($m[3]) && $m[3] !== '') {
                    $val = $m[3];
                } elseif (isset($m[4]) && $m[4] !== '') {
                    $val = $m[4];
                }
                $el->setAttribute($name, html_entity_decode($val, ENT_QUOTES | ENT_HTML5));
            }
        }
    }
}

class DOMXPath
{
    public $document;

    public function __construct(DOMDocument $doc)
    {
        $this->document = $doc;
    }

    public function query(string $expression, ?DOMNode $contextNode = null, bool $registerNodeNS = true): DOMNodeList
    {
        $expr = trim($expression);
        if ($expr === '') {
            return new DOMNodeList([]);
        }

        $startNode = $contextNode ?? $this->document->documentElement ?? $this->document;
        if (!$startNode) {
            return new DOMNodeList([]);
        }

        $isAbsolute = str_starts_with($expr, '/');
        $isDescendant = str_starts_with($expr, '//');

        if ($isDescendant) {
            $tag = trim(substr($expr, 2));
            $matched = [];
            $this->_collectDescendantsByTag($startNode, $tag, $matched);
            return new DOMNodeList($matched);
        }

        $parts = explode('/', trim($expr, '/'));
        $currentSet = [$startNode];

        if ($isAbsolute && $this->document && $this->document->documentElement) {
            $docEl = $this->document->documentElement;
            if (!empty($parts) && strcasecmp($parts[0], $docEl->tagName) === 0) {
                $currentSet = [$docEl];
                array_shift($parts);
            } else {
                $currentSet = [$docEl];
            }
        }

        foreach ($parts as $part) {
            $part = trim($part);
            if ($part === '' || $part === '.') {
                continue;
            }
            $nextSet = [];
            foreach ($currentSet as $node) {
                if ($node instanceof DOMDocument) {
                    if ($node->documentElement) {
                        if ($part === '*' || strcasecmp($node->documentElement->tagName, $part) === 0) {
                            $nextSet[] = $node->documentElement;
                        }
                    }
                } elseif ($node instanceof DOMNode && $node->childNodes) {
                    foreach ($node->childNodes->getNodes() as $child) {
                        if ($child instanceof DOMElement) {
                            if ($part === '*' || strcasecmp($child->tagName, $part) === 0) {
                                $nextSet[] = $child;
                            }
                        }
                    }
                }
            }
            $currentSet = $nextSet;
        }

        return new DOMNodeList($currentSet);
    }

    private function _collectDescendantsByTag(DOMNode $parent, string $tag, array &$result): void
    {
        if ($parent->childNodes) {
            foreach ($parent->childNodes->getNodes() as $child) {
                if ($child instanceof DOMElement) {
                    if ($tag === '*' || strcasecmp($child->tagName, $tag) === 0) {
                        $result[] = $child;
                    }
                    $this->_collectDescendantsByTag($child, $tag, $result);
                }
            }
        }
    }

    public function evaluate(string $expression, ?DOMNode $contextNode = null, bool $registerNodeNS = true): mixed
    {
        return $this->query($expression, $contextNode, $registerNodeNS);
    }

    public function registerNamespace(string $prefix, string $namespace): bool
    {
        return true;
    }
}

class NumberFormatter
{
    // Format styles
    public const PATTERN_DECIMAL = 0;
    public const DECIMAL = 1;
    public const CURRENCY = 2;
    public const PERCENT = 3;
    public const SCIENTIFIC = 4;
    public const SPELLOUT = 5;
    public const ORDINAL = 6;
    public const DURATION = 7;
    public const NUMBERING_SYSTEM = 8;
    public const PATTERN_RULEBASED = 9;
    public const CURRENCY_CODE = 10;
    public const DEFAULT_STYLE = 1;

    // Attributes
    public const PARSE_INT_ONLY = 0;
    public const GROUPING_USED = 1;
    public const DECIMAL_ALWAYS_SHOWN = 2;
    public const MAX_INTEGER_DIGITS = 3;
    public const MIN_INTEGER_DIGITS = 4;
    public const INTEGER_DIGITS = 5;
    public const MAX_FRACTION_DIGITS = 6;
    public const MIN_FRACTION_DIGITS = 7;
    public const FRACTION_DIGITS = 8;
    public const MULTIPLIER = 9;
    public const GROUPING_SIZE = 10;
    public const ROUNDING_MODE = 11;
    public const ROUNDING_INCREMENT = 12;
    public const FORMAT_WIDTH = 13;
    public const PADDING_POSITION = 14;
    public const SECONDARY_GROUPING_SIZE = 15;
    public const SIGNIFICANT_DIGITS_USED = 16;
    public const MIN_SIGNIFICANT_DIGITS = 17;
    public const MAX_SIGNIFICANT_DIGITS = 18;
    public const LENIENT_PARSE = 19;

    // Text attributes
    public const POSITIVE_PREFIX = 0;
    public const POSITIVE_SUFFIX = 1;
    public const NEGATIVE_PREFIX = 2;
    public const NEGATIVE_SUFFIX = 3;
    public const PADDING_CHARACTER = 4;
    public const CURRENCY_CODE_ATTR = 5;
    public const DEFAULT_RULESET = 6;
    public const PUBLIC_RULESETS = 7;

    // Types
    public const TYPE_DEFAULT = 1;
    public const TYPE_INT32 = 2;
    public const TYPE_INT64 = 3;
    public const TYPE_DOUBLE = 4;
    public const TYPE_CURRENCY = 5;

    // Rounding modes
    public const ROUND_CEILING = 0;
    public const ROUND_FLOOR = 1;
    public const ROUND_DOWN = 2;
    public const ROUND_UP = 3;
    public const ROUND_HALFEVEN = 4;
    public const ROUND_HALFDOWN = 5;
    public const ROUND_HALFUP = 6;

    private $locale;
    private $style;
    private $pattern;
    private $attributes = [];
    private $textAttributes = [];

    public function __construct($locale, $style, $pattern = '')
    {
        $this->locale = $locale;
        $this->style = $style;
        $this->pattern = $pattern;
        $this->attributes[self::GROUPING_USED] = 1;
        $this->attributes[self::FRACTION_DIGITS] = 0;
    }

    public static function create($locale, $style, $pattern = '')
    {
        return new static($locale, $style, $pattern);
    }

    public function setAttribute($attribute, $value)
    {
        $this->attributes[$attribute] = $value;
        return true;
    }

    public function getAttribute($attribute)
    {
        return $this->attributes[$attribute] ?? false;
    }

    public function setTextAttribute($attribute, $value)
    {
        $this->textAttributes[$attribute] = $value;
        return true;
    }

    public function getTextAttribute($attribute)
    {
        return $this->textAttributes[$attribute] ?? false;
    }

    public function format($num, $type = self::TYPE_DEFAULT)
    {
        $decimals = 0;
        if (isset($this->attributes[self::FRACTION_DIGITS])) {
            $decimals = (int)$this->attributes[self::FRACTION_DIGITS];
        } elseif (isset($this->attributes[self::MAX_FRACTION_DIGITS])) {
            $decimals = (int)$this->attributes[self::MAX_FRACTION_DIGITS];
        }

        $useGrouping = !isset($this->attributes[self::GROUPING_USED]) || $this->attributes[self::GROUPING_USED] != 0;
        $thousandsSep = $useGrouping ? ',' : '';
        $decPoint = '.';

        if ($this->style === self::CURRENCY) {
            $formatted = number_format((float)$num, $decimals ?: 2, $decPoint, $thousandsSep);
            return '$' . $formatted;
        } elseif ($this->style === self::PERCENT) {
            $formatted = number_format((float)$num * 100, $decimals, $decPoint, $thousandsSep);
            return $formatted . '%';
        } else {
            return number_format((float)$num, $decimals, $decPoint, $thousandsSep);
        }
    }

    public function formatCurrency($amount, $currency)
    {
        $decimals = isset($this->attributes[self::FRACTION_DIGITS]) ? (int)$this->attributes[self::FRACTION_DIGITS] : 2;
        $formatted = number_format((float)$amount, $decimals, '.', ',');
        return $currency . ' ' . $formatted;
    }

    public function parse($string, $type = self::TYPE_DOUBLE, &$offset = null)
    {
        $clean = str_replace(',', '', trim((string)$string));
        if ($type === self::TYPE_INT32 || $type === self::TYPE_INT64) {
            return (int)$clean;
        }
        return (float)$clean;
    }

    public function getErrorCode()
    {
        return 0;
    }

    public function getErrorMessage()
    {
        return 'U_ZERO_ERROR';
    }
}

