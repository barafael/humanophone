let wasm;

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

let WASM_VECTOR_LEN = 0;

let cachedUint8Memory0 = null;

function getUint8Memory0() {
    if (cachedUint8Memory0 === null || cachedUint8Memory0.byteLength === 0) {
        cachedUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8Memory0;
}

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length) >>> 0;
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len) >>> 0;

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3) >>> 0;
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachedInt32Memory0 = null;

function getInt32Memory0() {
    if (cachedInt32Memory0 === null || cachedInt32Memory0.byteLength === 0) {
        cachedInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachedInt32Memory0;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let cachedFloat64Memory0 = null;

function getFloat64Memory0() {
    if (cachedFloat64Memory0 === null || cachedFloat64Memory0.byteLength === 0) {
        cachedFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_2.get(state.dtor)(a, state.b);

            } else {
                state.a = a;
            }
        }
    };
    real.original = state;

    return real;
}

let stack_pointer = 128;

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}
function __wbg_adapter_24(arg0, arg1, arg2) {
    try {
        wasm._dyn_core__ops__function__FnMut___A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__ha90cb2a89e4ebc6d(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wbg_adapter_27(arg0, arg1, arg2) {
    try {
        wasm._dyn_core__ops__function__FnMut___A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h0542a5302c61a1a5(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wbg_adapter_30(arg0, arg1, arg2) {
    try {
        wasm._dyn_core__ops__function__FnMut___A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h99ee245727bda62a(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wbg_adapter_33(arg0, arg1, arg2) {
    wasm._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hce3e4e1d4eb5c12e(arg0, arg1, addHeapObject(arg2));
}

let cachedUint32Memory0 = null;

function getUint32Memory0() {
    if (cachedUint32Memory0 === null || cachedUint32Memory0.byteLength === 0) {
        cachedUint32Memory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32Memory0;
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getUint32Memory0();
    const slice = mem.subarray(ptr / 4, ptr / 4 + len);
    const result = [];
    for (let i = 0; i < slice.length; i++) {
        result.push(takeObject(slice[i]));
    }
    return result;
}
/**
* Sets the panic hook to print to the console.
*/
export function set_panic_hook() {
    wasm.set_panic_hook();
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
    return instance.ptr;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_exn_store(addHeapObject(e));
    }
}
/**
* The chord modifiers.
*/
export const KordModifier = Object.freeze({
/**
* Minor modifier.
*/
Minor:0,"0":"Minor",
/**
* Flat 5 modifier.
*/
Flat5:1,"1":"Flat5",
/**
* Sharp 5 modifier.
*/
Augmented5:2,"2":"Augmented5",
/**
* Major 7 modifier.
*/
Major7:3,"3":"Major7",
/**
* Dominant 7 modifier.
*/
Dominant7:4,"4":"Dominant7",
/**
* Dominant 9 modifier.
*/
Dominant9:5,"5":"Dominant9",
/**
* Dominant 11 modifier.
*/
Dominant11:6,"6":"Dominant11",
/**
* Dominant 13 modifier.
*/
Dominant13:7,"7":"Dominant13",
/**
* Flat 9 modifier.
*/
Flat9:8,"8":"Flat9",
/**
* Sharp 9 modifier.
*/
Sharp9:9,"9":"Sharp9",
/**
* Sharp 11 modifier.
*/
Sharp11:10,"10":"Sharp11",
/**
* Diminished modifier.
*/
Diminished:11,"11":"Diminished", });
/**
* An enum representing the interval between two notes.
*/
export const KordInterval = Object.freeze({
/**
* A perfect unison interval.
*/
PerfectUnison:0,"0":"PerfectUnison",
/**
* A diminished second interval.
*/
DiminishedSecond:1,"1":"DiminishedSecond",
/**
* An augmented unison interval.
*/
AugmentedUnison:2,"2":"AugmentedUnison",
/**
* A minor second interval.
*/
MinorSecond:3,"3":"MinorSecond",
/**
* A major second interval.
*/
MajorSecond:4,"4":"MajorSecond",
/**
* A diminished third interval.
*/
DiminishedThird:5,"5":"DiminishedThird",
/**
* An augmented second interval.
*/
AugmentedSecond:6,"6":"AugmentedSecond",
/**
* A minor third interval.
*/
MinorThird:7,"7":"MinorThird",
/**
* A major third interval.
*/
MajorThird:8,"8":"MajorThird",
/**
* A diminished fourth interval.
*/
DiminishedFourth:9,"9":"DiminishedFourth",
/**
* An augmented third interval.
*/
AugmentedThird:10,"10":"AugmentedThird",
/**
* A perfect fourth interval.
*/
PerfectFourth:11,"11":"PerfectFourth",
/**
* An augmented fourth interval.
*/
AugmentedFourth:12,"12":"AugmentedFourth",
/**
* A diminished fifth interval.
*/
DiminishedFifth:13,"13":"DiminishedFifth",
/**
* A perfect fifth interval.
*/
PerfectFifth:14,"14":"PerfectFifth",
/**
* A diminished sixth interval.
*/
DiminishedSixth:15,"15":"DiminishedSixth",
/**
* An augmented fifth interval.
*/
AugmentedFifth:16,"16":"AugmentedFifth",
/**
* A minor sixth interval.
*/
MinorSixth:17,"17":"MinorSixth",
/**
* A major sixth interval.
*/
MajorSixth:18,"18":"MajorSixth",
/**
* A diminished seventh interval.
*/
DiminishedSeventh:19,"19":"DiminishedSeventh",
/**
* An augmented sixth interval.
*/
AugmentedSixth:20,"20":"AugmentedSixth",
/**
* A minor seventh interval.
*/
MinorSeventh:21,"21":"MinorSeventh",
/**
* A major seventh interval.
*/
MajorSeventh:22,"22":"MajorSeventh",
/**
* A diminished octave interval.
*/
DiminishedOctave:23,"23":"DiminishedOctave",
/**
* An augmented seventh interval.
*/
AugmentedSeventh:24,"24":"AugmentedSeventh",
/**
* A perfect octave interval.
*/
PerfectOctave:25,"25":"PerfectOctave",
/**
* An minor ninth interval.
*/
MinorNinth:26,"26":"MinorNinth",
/**
* A major ninth interval.
*/
MajorNinth:27,"27":"MajorNinth",
/**
* An augmented ninth interval.
*/
AugmentedNinth:28,"28":"AugmentedNinth",
/**
* A diminished eleventh interval.
*/
DiminishedEleventh:29,"29":"DiminishedEleventh",
/**
* A perfect eleventh interval.
*/
PerfectEleventh:30,"30":"PerfectEleventh",
/**
* An augmented eleventh interval.
*/
AugmentedEleventh:31,"31":"AugmentedEleventh",
/**
* A minor thirteenth interval.
*/
MinorThirteenth:32,"32":"MinorThirteenth",
/**
* A major thirteenth interval.
*/
MajorThirteenth:33,"33":"MajorThirteenth",
/**
* An augmented thirteenth interval.
*/
AugmentedThirteenth:34,"34":"AugmentedThirteenth",
/**
* A perfect octave and perfect fifth interval.
*/
PerfectOctaveAndPerfectFifth:35,"35":"PerfectOctaveAndPerfectFifth",
/**
* Two perfect octaves.
*/
TwoPerfectOctaves:36,"36":"TwoPerfectOctaves",
/**
* Two perfect octaves and a major third.
*/
TwoPerfectOctavesAndMajorThird:37,"37":"TwoPerfectOctavesAndMajorThird",
/**
* Two perfect octaves and a perfect fifth.
*/
TwoPerfectOctavesAndPerfectFifth:38,"38":"TwoPerfectOctavesAndPerfectFifth",
/**
* Two perfect octaves and a minor sixth.
*/
TwoPerfectOctavesAndMinorSeventh:39,"39":"TwoPerfectOctavesAndMinorSeventh",
/**
* Three perfect octaves.
*/
ThreePerfectOctaves:40,"40":"ThreePerfectOctaves",
/**
* Three perfect octaves and a major second.
*/
ThreePerfectOctavesAndMajorSecond:41,"41":"ThreePerfectOctavesAndMajorSecond",
/**
* Three perfect octaves and a major third.
*/
ThreePerfectOctavesAndMajorThird:42,"42":"ThreePerfectOctavesAndMajorThird",
/**
* Three perfect octaves and an augmented fourth.
*/
ThreePerfectOctavesAndAugmentedFourth:43,"43":"ThreePerfectOctavesAndAugmentedFourth",
/**
* Three perfect octaves and a perfect fifth.
*/
ThreePerfectOctavesAndPerfectFifth:44,"44":"ThreePerfectOctavesAndPerfectFifth",
/**
* Three perfect octaves and a minor sixth.
*/
ThreePerfectOctavesAndMinorSixth:45,"45":"ThreePerfectOctavesAndMinorSixth",
/**
* Three perfect octaves and a minor seventh.
*/
ThreePerfectOctavesAndMinorSeventh:46,"46":"ThreePerfectOctavesAndMinorSeventh",
/**
* Three perfect octaves and a major seventh.
*/
ThreePerfectOctavesAndMajorSeventh:47,"47":"ThreePerfectOctavesAndMajorSeventh", });
/**
* An enum representing the extension of a chord.
*
* Extensions are not really "special" in the sense that they do not change how the
* chord is interpreted by the system.  E.g., an `add2` just adds a 2 to the chord,
* and the chord is still interpreted as a major chord.
*/
export const KordExtension = Object.freeze({
/**
* Sus2 extension.
*/
Sus2:0,"0":"Sus2",
/**
* Sus4 extension.
*/
Sus4:1,"1":"Sus4",
/**
* Flat 11 extension.
*/
Flat11:2,"2":"Flat11",
/**
* Flat 13 extension.
*/
Flat13:3,"3":"Flat13",
/**
* Sharp 13 extension.
*/
Sharp13:4,"4":"Sharp13",
/**
* Add2 extension.
*/
Add2:5,"5":"Add2",
/**
* Add4 extension.
*/
Add4:6,"6":"Add4",
/**
* Add6 extension.
*/
Add6:7,"7":"Add6",
/**
* Add9 extension.
*/
Add9:8,"8":"Add9",
/**
* Add11 extension.
*/
Add11:9,"9":"Add11",
/**
* Add13 extension.
*/
Add13:10,"10":"Add13", });
/**
* The [`Chord`] wrapper.
*/
export class KordChord {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(KordChord.prototype);
        obj.__wbg_ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_kordchord_free(ptr);
    }
    /**
    * Creates a new [`Chord`] from a frequency.
    * @param {string} name
    * @returns {KordChord}
    */
    static parse(name) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.kordchord_parse(retptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return KordChord.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Creates a new [`Chord`] from a set of [`Note`]s.
    *
    * The [`Note`]s should be represented as a space-separated string.
    * E.g., `C E G`.
    * @param {string} notes
    * @returns {Array<any>}
    */
    static fromNotesString(notes) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(notes, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.kordchord_fromNotesString(retptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Creates a new [`Chord`] from a set of [`Note`]s.
    * @param {Array<any>} notes
    * @returns {Array<any>}
    */
    static fromNotes(notes) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_fromNotes(retptr, addHeapObject(notes));
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return takeObject(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Returns the [`Chord`]'s friendly name.
    * @returns {string}
    */
    name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_name(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s precise name.
    * @returns {string}
    */
    preciseName() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_preciseName(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`] as a string (same as `precise_name`).
    * @returns {string}
    */
    toString() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_preciseName(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s description.
    * @returns {string}
    */
    description() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_description(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s display text.
    * @returns {string}
    */
    display() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_display(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s root note.
    * @returns {string}
    */
    root() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_root(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s slash note.
    * @returns {string}
    */
    slash() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_slash(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s inversion.
    * @returns {number}
    */
    inversion() {
        const ret = wasm.kordchord_inversion(this.__wbg_ptr);
        return ret;
    }
    /**
    * Returns whether or not the [`Chord`] is "crunchy".
    * @returns {boolean}
    */
    isCrunchy() {
        const ret = wasm.kordchord_isCrunchy(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
    * Returns the [`Chord`]'s chord tones.
    * @returns {Array<any>}
    */
    chord() {
        const ret = wasm.kordchord_chord(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * Returns the [`Chord`]'s chord tones as a string.
    * @returns {string}
    */
    chordString() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_chordString(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s scale tones.
    * @returns {Array<any>}
    */
    scale() {
        const ret = wasm.kordchord_scale(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * Returns the [`Chord`]'s scale tones as a string.
    * @returns {string}
    */
    scaleString() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_scaleString(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Chord`]'s modifiers.
    * @returns {Array<any>}
    */
    modifiers() {
        const ret = wasm.kordchord_modifiers(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * Returns the [`Chord`]'s extensions.
    * @returns {Array<any>}
    */
    extensions() {
        const ret = wasm.kordchord_extensions(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * Returns a new [`Chord`] with the inversion set to the provided value.
    * @param {number} inversion
    * @returns {KordChord}
    */
    withInversion(inversion) {
        const ret = wasm.kordchord_withInversion(this.__wbg_ptr, inversion);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the slash set to the provided value.
    * @param {KordNote} slash
    * @returns {KordChord}
    */
    withSlash(slash) {
        _assertClass(slash, KordNote);
        const ret = wasm.kordchord_withSlash(this.__wbg_ptr, slash.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the octave of the root set to the provided value.
    * @param {number} octave
    * @returns {KordChord}
    */
    withOctave(octave) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordchord_withOctave(retptr, this.__wbg_ptr, octave);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return KordChord.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Returns a new [`Chord`] with the "crunchiness" set to the provided value.
    * @param {boolean} is_crunchy
    * @returns {KordChord}
    */
    withCrunchy(is_crunchy) {
        const ret = wasm.kordchord_withCrunchy(this.__wbg_ptr, is_crunchy);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns the clone of the [`Chord`].
    * @returns {KordChord}
    */
    copy() {
        const ret = wasm.kordchord_copy(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `minor` modifier.
    * @returns {KordChord}
    */
    minor() {
        const ret = wasm.kordchord_minor(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `flat5` modifier.
    * @returns {KordChord}
    */
    flat5() {
        const ret = wasm.kordchord_flat5(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `augmented` modifier.
    * @returns {KordChord}
    */
    aug() {
        const ret = wasm.kordchord_aug(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `maj7` modifier.
    * @returns {KordChord}
    */
    maj7() {
        const ret = wasm.kordchord_maj7(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `dom7` modifier.
    * @returns {KordChord}
    */
    seven() {
        const ret = wasm.kordchord_seven(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `dom9` modifier.
    * @returns {KordChord}
    */
    nine() {
        const ret = wasm.kordchord_nine(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `dom11` modifier.
    * @returns {KordChord}
    */
    eleven() {
        const ret = wasm.kordchord_eleven(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `dom13` modifier.
    * @returns {KordChord}
    */
    thirteen() {
        const ret = wasm.kordchord_thirteen(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `flat9` modifier.
    * @returns {KordChord}
    */
    flat9() {
        const ret = wasm.kordchord_flat9(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `sharp9` modifier.
    * @returns {KordChord}
    */
    sharp9() {
        const ret = wasm.kordchord_sharp9(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `sharp11` modifier.
    * @returns {KordChord}
    */
    sharp11() {
        const ret = wasm.kordchord_sharp11(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `dim` modifier.
    * @returns {KordChord}
    */
    dim() {
        const ret = wasm.kordchord_dim(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `halfDim` modifier.
    * @returns {KordChord}
    */
    halfDim() {
        const ret = wasm.kordchord_halfDim(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `sus2` extension.
    * @returns {KordChord}
    */
    sus2() {
        const ret = wasm.kordchord_sus2(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `sus4` extension.
    * @returns {KordChord}
    */
    sus4() {
        const ret = wasm.kordchord_sus4(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `flat11` extension.
    * @returns {KordChord}
    */
    flat11() {
        const ret = wasm.kordchord_flat11(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `flat13` extension.
    * @returns {KordChord}
    */
    flat13() {
        const ret = wasm.kordchord_flat13(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `sharp13` extension.
    * @returns {KordChord}
    */
    sharp13() {
        const ret = wasm.kordchord_sharp13(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `add2` extension.
    * @returns {KordChord}
    */
    add2() {
        const ret = wasm.kordchord_add2(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `add4` extension.
    * @returns {KordChord}
    */
    add4() {
        const ret = wasm.kordchord_add4(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `add6` extension.
    * @returns {KordChord}
    */
    add6() {
        const ret = wasm.kordchord_add6(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `add9` extension.
    * @returns {KordChord}
    */
    add9() {
        const ret = wasm.kordchord_add9(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `add11` extension.
    * @returns {KordChord}
    */
    add11() {
        const ret = wasm.kordchord_add11(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
    /**
    * Returns a new [`Chord`] with the `add13` extension.
    * @returns {KordChord}
    */
    add13() {
        const ret = wasm.kordchord_add13(this.__wbg_ptr);
        return KordChord.__wrap(ret);
    }
}
/**
* The [`Note`] wrapper.
*/
export class KordNote {

    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(KordNote.prototype);
        obj.__wbg_ptr = ptr;

        return obj;
    }

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_kordnote_free(ptr);
    }
    /**
    * Creates a new [`Note`] from a frequency.
    * @param {string} name
    * @returns {KordNote}
    */
    static parse(name) {
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            wasm.kordnote_parse(retptr, ptr0, len0);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            var r2 = getInt32Memory0()[retptr / 4 + 2];
            if (r2) {
                throw takeObject(r1);
            }
            return KordNote.__wrap(r0);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
        }
    }
    /**
    * Returns the [`Note`]'s friendly name.
    * @returns {string}
    */
    name() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordnote_name(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Note`] represented as a string (same as `name`).
    * @returns {string}
    */
    toString() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordnote_toString(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Note`]'s [`NamedPitch`].
    * @returns {string}
    */
    pitch() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.kordnote_pitch(retptr, this.__wbg_ptr);
            var r0 = getInt32Memory0()[retptr / 4 + 0];
            var r1 = getInt32Memory0()[retptr / 4 + 1];
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_free(deferred1_0, deferred1_1);
        }
    }
    /**
    * Returns the [`Note`]'s [`Octave`].
    * @returns {number}
    */
    octave() {
        const ret = wasm.kordnote_octave(this.__wbg_ptr);
        return ret;
    }
    /**
    * Returns the [`Note`]'s frequency.
    * @returns {number}
    */
    frequency() {
        const ret = wasm.kordnote_frequency(this.__wbg_ptr);
        return ret;
    }
    /**
    * Adds the given interval to the [`Note`], producing a new [`Note`] instance.
    * @param {number} interval
    * @returns {KordNote}
    */
    addInterval(interval) {
        const ret = wasm.kordnote_addInterval(this.__wbg_ptr, interval);
        return KordNote.__wrap(ret);
    }
    /**
    * Subtracts the given interval from the [`Note`], producing a new [`Note`] instance.
    * @param {number} interval
    * @returns {KordNote}
    */
    subInterval(interval) {
        const ret = wasm.kordnote_subInterval(this.__wbg_ptr, interval);
        return KordNote.__wrap(ret);
    }
    /**
    * Computes the [`Interval`] distance between the [`Note`] and the given [`Note`].
    * @param {KordNote} other
    * @returns {number}
    */
    distanceTo(other) {
        _assertClass(other, KordNote);
        var ptr0 = other.__destroy_into_raw();
        const ret = wasm.kordnote_distanceTo(this.__wbg_ptr, ptr0);
        return ret >>> 0;
    }
    /**
    * Returns the primary (first 13) harmonic series of the [`Note`].
    * @returns {Array<any>}
    */
    harmonicSeries() {
        const ret = wasm.kordnote_harmonicSeries(this.__wbg_ptr);
        return takeObject(ret);
    }
    /**
    * Returns the clone of the [`Note`].
    * @returns {KordNote}
    */
    copy() {
        const ret = wasm.kordnote_copy(this.__wbg_ptr);
        return KordNote.__wrap(ret);
    }
}
/**
* A handle to a [`Chord`] playback.
*
* Should be dropped to stop the playback, or after playback is finished.
*/
export class KordPlaybackHandle {

    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;

        return ptr;
    }

    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_kordplaybackhandle_free(ptr);
    }
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_is_string = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'string';
        return ret;
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbg_cachekey_b61393159c57fd7b = function(arg0, arg1) {
        const ret = getObject(arg1).__yew_subtree_cache_key;
        getInt32Memory0()[arg0 / 4 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_subtreeid_e348577f7ef777e3 = function(arg0, arg1) {
        const ret = getObject(arg1).__yew_subtree_id;
        getInt32Memory0()[arg0 / 4 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_setsubtreeid_d32e6327eef1f7fc = function(arg0, arg1) {
        getObject(arg0).__yew_subtree_id = arg1 >>> 0;
    };
    imports.wbg.__wbg_setcachekey_80183b7cfc421143 = function(arg0, arg1) {
        getObject(arg0).__yew_subtree_cache_key = arg1 >>> 0;
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_listenerid_12315eee21527820 = function(arg0, arg1) {
        const ret = getObject(arg1).__yew_listener_id;
        getInt32Memory0()[arg0 / 4 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_setlistenerid_3183aae8fa5840fb = function(arg0, arg1) {
        getObject(arg0).__yew_listener_id = arg1 >>> 0;
    };
    imports.wbg.__wbg_error_71d6845bf00a930f = function(arg0, arg1) {
        var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
        wasm.__wbindgen_free(arg0, arg1 * 4);
        console.error(...v0);
    };
    imports.wbg.__wbg_kordnote_new = function(arg0) {
        const ret = KordNote.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_kordchord_new = function(arg0) {
        const ret = KordChord.__wrap(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_new_abda76e883ba8a5f = function() {
        const ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_stack_658279fe44541cf6 = function(arg0, arg1) {
        const ret = getObject(arg1).stack;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_error_f851667af71bcfc6 = function(arg0, arg1) {
        let deferred0_0;
        let deferred0_1;
        try {
            deferred0_0 = arg0;
            deferred0_1 = arg1;
            console.error(getStringFromWasm0(arg0, arg1));
        } finally {
            wasm.__wbindgen_free(deferred0_0, deferred0_1);
        }
    };
    imports.wbg.__wbg_body_db30cc67afcfce41 = function(arg0) {
        const ret = getObject(arg0).body;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_createElement_d975e66d06bc88da = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createElementNS_0863d6a8a49df376 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).createElementNS(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createTextNode_31876ed40128c33c = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).createTextNode(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_Window_c5579e140698a9dc = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Window;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_document_508774c021174a52 = function(arg0) {
        const ret = getObject(arg0).document;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_ShadowRoot_5aea367fb03b2fff = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof ShadowRoot;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_host_ca6efb67ffed0f2e = function(arg0) {
        const ret = getObject(arg0).host;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_readyState_ad4a43cf245ed346 = function(arg0) {
        const ret = getObject(arg0).readyState;
        return ret;
    };
    imports.wbg.__wbg_setbinaryType_912a58ff49a07245 = function(arg0, arg1) {
        getObject(arg0).binaryType = takeObject(arg1);
    };
    imports.wbg.__wbg_new_c70a4fdc1ed8f3bb = function() { return handleError(function (arg0, arg1) {
        const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_close_b57426e46b2cde05 = function() { return handleError(function (arg0) {
        getObject(arg0).close();
    }, arguments) };
    imports.wbg.__wbg_send_1cc5e505b0cbc15c = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).send(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_value_664b8ba8bd4419b0 = function(arg0, arg1) {
        const ret = getObject(arg1).value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_setvalue_272abbd8c7ff3573 = function(arg0, arg1, arg2) {
        getObject(arg0).value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setchecked_46f40fa426cedbb8 = function(arg0, arg1) {
        getObject(arg0).checked = arg1 !== 0;
    };
    imports.wbg.__wbg_value_09d384cba1c51c6f = function(arg0, arg1) {
        const ret = getObject(arg1).value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_setvalue_7605619324f70225 = function(arg0, arg1, arg2) {
        getObject(arg0).value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_instanceof_Element_6fe31b975e43affc = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Element;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_namespaceURI_a1c6e4b9bb827959 = function(arg0, arg1) {
        const ret = getObject(arg1).namespaceURI;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_setinnerHTML_76dc2e7ffb1c1936 = function(arg0, arg1, arg2) {
        getObject(arg0).innerHTML = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_outerHTML_e90651c874c31e05 = function(arg0, arg1) {
        const ret = getObject(arg1).outerHTML;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_children_a62d21390b1805ff = function(arg0) {
        const ret = getObject(arg0).children;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_removeAttribute_77e4f460fd0fde34 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).removeAttribute(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_setAttribute_1b177bcd399b9b56 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_instanceof_MessageEvent_504f64ff31f4aaf2 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof MessageEvent;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_data_338d13609f6165a6 = function(arg0) {
        const ret = getObject(arg0).data;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_close_c4da68c7d05f0953 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).close();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_bubbles_0a277ad42caf0211 = function(arg0) {
        const ret = getObject(arg0).bubbles;
        return ret;
    };
    imports.wbg.__wbg_cancelBubble_42441ef40999b550 = function(arg0) {
        const ret = getObject(arg0).cancelBubble;
        return ret;
    };
    imports.wbg.__wbg_composedPath_85d84e53cceb3d62 = function(arg0) {
        const ret = getObject(arg0).composedPath();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_parentNode_65dd881ebb22f646 = function(arg0) {
        const ret = getObject(arg0).parentNode;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_parentElement_065722829508e41a = function(arg0) {
        const ret = getObject(arg0).parentElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_lastChild_649563f43d5b930d = function(arg0) {
        const ret = getObject(arg0).lastChild;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_nextSibling_6e2efeefd07e6f9e = function(arg0) {
        const ret = getObject(arg0).nextSibling;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_setnodeValue_008911a41f1b91a3 = function(arg0, arg1, arg2) {
        getObject(arg0).nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_textContent_d953d0aec79e1ba6 = function(arg0, arg1) {
        const ret = getObject(arg1).textContent;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbg_appendChild_1139b53a65d69bed = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).appendChild(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_insertBefore_2e38a68009b551f3 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).insertBefore(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_removeChild_48d9566cffdfec93 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).removeChild(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_addEventListener_3a7d7c4177ce91d1 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
    }, arguments) };
    imports.wbg.__wbg_removeEventListener_315d6f929fccf484 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).removeEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), arg4 !== 0);
    }, arguments) };
    imports.wbg.__wbg_get_7303ed2ef026b2f5 = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_length_820c786973abdd8a = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_new_0394642eae39db16 = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_newnoargs_c9e6043b8ad84109 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_get_f53c921291c381bd = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_557a2f2deacc4912 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_2b6fea4ea03b1b95 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_self_742dd6eab3e9211e = function() { return handleError(function () {
        const ret = self.self;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_window_c409e731db53a0e2 = function() { return handleError(function () {
        const ret = window.window;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_globalThis_b70c095388441f2d = function() { return handleError(function () {
        const ret = globalThis.globalThis;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_global_1c72617491ed7194 = function() { return handleError(function () {
        const ret = global.global;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbg_from_6bc98a09a0b58bb1 = function(arg0) {
        const ret = Array.from(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_push_109cfc26d02582dd = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_toString_506566b763774a16 = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_name_2a8bae31363c6a51 = function(arg0) {
        const ret = getObject(arg0).name;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_Object_a9e9e5766628e8b5 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Object;
        } catch {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_constructor_f2623999a1f453eb = function(arg0) {
        const ret = getObject(arg0).constructor;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_is_20a2e5c82eecc47d = function(arg0, arg1) {
        const ret = Object.is(getObject(arg0), getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_resolve_ae38ad63c43ff98b = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_8df675b8bb5d5e3c = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_buffer_55ba7a6b1b92e2ac = function(arg0) {
        const ret = getObject(arg0).buffer;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_09938a7d020f049b = function(arg0) {
        const ret = new Uint8Array(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_3698e3ca519b3c3c = function(arg0, arg1, arg2) {
        getObject(arg0).set(getObject(arg1), arg2 >>> 0);
    };
    imports.wbg.__wbg_length_0aab7ffd65ad19ed = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_set_07da13cc24b69217 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };
    imports.wbg.__wbindgen_memory = function() {
        const ret = wasm.memory;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper305 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 84, __wbg_adapter_24);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper371 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 121, __wbg_adapter_27);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper474 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 157, __wbg_adapter_30);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper722 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 250, __wbg_adapter_33);
        return addHeapObject(ret);
    };

    return imports;
}

function __wbg_init_memory(imports, maybe_memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedFloat64Memory0 = null;
    cachedInt32Memory0 = null;
    cachedUint32Memory0 = null;
    cachedUint8Memory0 = null;

    wasm.__wbindgen_start();
    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(input) {
    if (wasm !== undefined) return wasm;

    if (typeof input === 'undefined') {
        input = new URL('horeau-5ed8fea29c3483cc_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
        input = fetch(input);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await input, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync }
export default __wbg_init;
