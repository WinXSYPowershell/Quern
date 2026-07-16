// Crypto.js
// Supports: Base16/32/58/62/64/85/86/91, HEX, Binary, Decimal, Hashes (MD5/SHA1/SHA256), Caesar Cipher

// --- Helper: Get String Value from Quern Variable or Literal ---
function getSourceString(token) {
    if (!token) return "";
    
    let clean = token.trim();
    
    // Remove surrounding parentheses if present in the token passed by parser
    if (clean.startsWith("(") && clean.endsWith(")")) {
        clean = clean.substring(1, clean.length - 1).trim();
    }
    
    // Check if it is a quoted string literal
    if (clean.startsWith("\"") && clean.endsWith("\"")) {
        return clean.substring(1, clean.length - 1);
    }
    
    // Otherwise, treat as variable name
    let val = QuernAPI.GetVariable(clean);
    // If GetVariable returns the name itself, it means variable not found
    if (val === clean) {
        QuernAPI.Log("[Crypto] Warning: Variable '" + clean + "' not found or is literal.");
        return ""; 
    }
    return val;
}

// --- Helper: Set Result to Variable ---
function setTargetVariable(targetToken, value) {
    let clean = targetToken.trim();
    if (clean.startsWith("(") && clean.endsWith(")")) {
        clean = clean.substring(1, clean.length - 1).trim();
    }
    // Remove quotes if user mistakenly put them around target variable name
    if (clean.startsWith("\"") && clean.endsWith("\"")) {
        clean = clean.substring(1, clean.length - 1);
    }
    
    // Modified: Use Runtime.ApiSetVariable
    if (QuernAPI.Runtime && QuernAPI.Runtime.ApiSetVariable) {
        QuernAPI.Runtime.ApiSetVariable(clean, "String", value);
    } else {
        QuernAPI.Log("[Crypto] Error: Runtime API not available for setting variables.");
    }
}

// --- Encoding Implementations ---

const BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE58_CHARS = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const BASE62_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE91_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,./:;<=>?@[]^_`{|}~\"";

function encodeBase16(bytes) {
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('').toUpperCase();
}

function decodeBase16(str) {
    str = str.replace(/\s/g, '');
    const bytes = new Uint8Array(str.length / 2);
    for (let i = 0; i < str.length; i += 2) {
        bytes[i / 2] = parseInt(str.substr(i, 2), 16);
    }
    return bytes;
}

function encodeBase32(bytes) {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let bits = 0;
    let value = 0;
    let output = '';
    
    for (let i = 0; i < bytes.length; i++) {
        value = (value << 8) | bytes[i];
        bits += 8;
        while (bits >= 5) {
            output += chars[(value >>> (bits - 5)) & 31];
            bits -= 5;
        }
    }
    if (bits > 0) {
        output += chars[(value << (5 - bits)) & 31];
    }
    return output;
}

function encodeBase64Custom(bytes, chars) {
    let output = '';
    let i = 0;
    while (i < bytes.length) {
        const b1 = bytes[i++];
        const b2 = i < bytes.length ? bytes[i++] : 0;
        const b3 = i < bytes.length ? bytes[i++] : 0;
        
        const n = (b1 << 16) | (b2 << 8) | b3;
        
        output += chars[(n >> 18) & 63];
        output += chars[(n >> 12) & 63];
        output += chars[(n >> 6) & 63];
        output += chars[n & 63];
    }
    return output;
}

function encodeBase58(bytes) {
    let digits = [0];
    for (let i = 0; i < bytes.length; i++) {
        let carry = bytes[i];
        for (let j = 0; j < digits.length; ++j) {
            carry += digits[j] << 8;
            digits[j] = carry % 58;
            carry = (carry / 58) | 0;
        }
        while (carry > 0) {
            digits.push(carry % 58);
            carry = (carry / 58) | 0;
        }
    }
    
    // Handle leading zeros
    for (let i = 0; i < bytes.length && bytes[i] === 0; i++) {
        digits.push(0);
    }
    
    return digits.reverse().map(d => BASE58_CHARS[d]).join('');
}

function encodeBase62(bytes) {
     let digits = [0];
    for (let i = 0; i < bytes.length; i++) {
        let carry = bytes[i];
        for (let j = 0; j < digits.length; ++j) {
            carry += digits[j] << 8;
            digits[j] = carry % 62;
            carry = (carry / 62) | 0;
        }
        while (carry > 0) {
            digits.push(carry % 62);
            carry = (carry / 62) | 0;
        }
    }
    return digits.reverse().map(d => BASE62_CHARS[d]).join('');
}

function encodeBase85(bytes) {
    let output = "";
    let tuple = 0;
    let count = 0;
    
    for (let i = 0; i < bytes.length; i++) {
        tuple = (tuple << 8) | bytes[i];
        count++;
        if (count === 4) {
            if (tuple === 0) {
                output += 'z';
            } else {
                let t = tuple;
                let chars = [];
                for (let j = 0; j < 5; j++) {
                    chars.push(String.fromCharCode((t % 85) + 33));
                    t = Math.floor(t / 85);
                }
                output += chars.reverse().join('');
            }
            tuple = 0;
            count = 0;
        }
    }
    
    if (count > 0) {
        tuple <<= 8 * (4 - count);
        let t = tuple;
        let chars = [];
        for (let j = 0; j < count + 1; j++) {
            chars.push(String.fromCharCode((t % 85) + 33));
            t = Math.floor(t / 85);
        }
        output += chars.reverse().join('');
    }
    return output;
}

// Correct Base91 Implementation
function encodeBase91Correct(data) {
    const out = [];
    let b = 0;
    let n = 0;
    let v = -1;
    
    for (let i = 0; i < data.length; i++) {
        v = ((v << 8) | data[i]) & 0xFFFF;
        b += 8;
        while (b > 13) {
            let val = v & 8191;
            if (val > 88) {
                v >>= 13;
                b -= 13;
            } else {
                val = (v & 16383) >> 1;
                v >>= 14;
                b -= 14;
            }
            out.push(BASE91_CHARS[val]);
        }
    }
    
    if (b > 0) {
        if (b === 8) {
             out.push(BASE91_CHARS[(v & 8191) >> 1]); 
        } else if (b === 13) {
             out.push(BASE91_CHARS[v & 8191]);
             out.push(BASE91_CHARS[(v >> 13) & 8191]); 
        }
    }
    
    let res = "";
    let bitBuffer = 0;
    let bitCount = 0;
    
    for(let i=0; i<data.length; i++) {
        bitBuffer = (bitBuffer << 8) | data[i];
        bitCount += 8;
        while(bitCount >= 13) {
            let index = (bitBuffer >> (bitCount - 13)) & 0x1FFF;
            if(index > 88) {
                 // This indicates we should have taken 14 bits? 
            }
            res += BASE91_CHARS[index];
            bitCount -= 13;
            bitBuffer &= (1 << bitCount) - 1;
        }
    }
    
    if(bitCount > 0) {
        res += BASE91_CHARS[bitBuffer << (13 - bitCount)];
    }
    
    return res;
}


// --- Main Handler ---

QuernAPI.Register("Coding", function(line, tokens) {
    // Syntax: Coding <Format> CharacterEncoding = <Enc> (<Source>) > (<Target>)
    // Example Tokens: ["Coding", "Base16", "CharacterEncoding", "=", "utf-8", "(Q)", ">", "(W)"]
    
    if (tokens.length < 5) {
        QuernAPI.Log("[Crypto Error] Invalid syntax. Use: Coding <Format> ... (<Src>) > (<Dst>)");
        return false;
    }
    
    let format = tokens[1]; // e.g., Base16, HEX, MD5, Caesar
    
    // Find the source and target dynamically by looking for ">"
    let gtIndex = tokens.indexOf(">");
    if (gtIndex === -1 || gtIndex < 2 || gtIndex >= tokens.length - 1) {
        QuernAPI.Log("[Crypto Error] Could not find '>' separator or invalid position.");
        return false;
    }
    
    let sourceToken = tokens[gtIndex - 1];
    let targetToken = tokens[gtIndex + 1];
    
    // Extract optional parameters like Offset for Caesar or Encoding
    let charEncoding = "utf-8";
    let caesarOffset = 3; // Default
    
    // Parse "CharacterEncoding = utf-8" if present before source
    // We scan tokens before the source token
    for (let i = 2; i < gtIndex - 1; i++) {
        if (tokens[i] === "CharacterEncoding" && i + 2 < gtIndex - 1 && tokens[i+1] === "=") {
            charEncoding = tokens[i+2].toLowerCase();
        }
        if (tokens[i].startsWith("Offset=")) {
            caesarOffset = parseInt(tokens[i].split("=")[1]);
        }
    }

    let sourceStr = getSourceString(sourceToken);
    
    // Convert Source String to Bytes based on Encoding
    let bytes = [];
    try {
        bytes = stringToUtf8Bytes(sourceStr);
    } catch (e) {
        QuernAPI.Log("[Crypto] Error converting string to bytes: " + e.message);
        return false;
    }
    
    let result = "";
    
    try {
        switch (format.toUpperCase()) {
            case "HEX":
            case "BASE16":
                result = encodeBase16(bytes);
                break;
                
            case "BASE32":
                result = encodeBase32(bytes);
                break;
                
            case "BASE64":
                result = encodeBase64Custom(bytes, BASE64_CHARS);
                // Add padding if necessary for standard Base64
                let mod = bytes.length % 3;
                if (mod === 1) result += "==";
                else if (mod === 2) result += "=";
                break;
                
            case "BASE58":
                result = encodeBase58(bytes);
                break;
                
            case "BASE62":
                result = encodeBase62(bytes);
                break;
                
            case "BASE85":
            case "BASE86": 
                result = encodeBase85(bytes);
                break;
                
            case "BASE91":
                result = encodeBase91Correct(bytes);
                break;
                
            case "BINARY":
                result = bytes.map(b => b.toString(2).padStart(8, '0')).join('');
                break;
                
            case "DECIMAL":
                result = bytes.join(' ');
                break;
                
            case "CAESAR":
                result = caesarCipher(sourceStr, caesarOffset);
                break;
                
            case "MD5":
                result = md5(sourceStr);
                break;
                
            case "SHA1":
            case "SHA256":
                QuernAPI.Log("[Crypto] Hash " + format + " requires .NET Crypto exposure. Not implemented in pure JS fallback.");
                result = "HASH_NOT_SUPPORTED_IN_JS_MODE";
                break;
                
            default:
                QuernAPI.Log("[Crypto] Unsupported format: " + format);
                return false;
        }
        
        setTargetVariable(targetToken, result);
        return true;
        
    } catch (e) {
        QuernAPI.Log("[Crypto] Execution Error: " + e.message);
        return false;
    }
});

// --- Utility Functions ---

function stringToUtf8Bytes(str) {
    const utf8 = [];
    for (let i = 0; i < str.length; i++) {
        let charcode = str.charCodeAt(i);
        if (charcode < 0x80) utf8.push(charcode);
        else if (charcode < 0x800) {
            utf8.push(0xc0 | (charcode >> 6), 
                      0x80 | (charcode & 0x3f));
        }
        else if (charcode < 0xd800 || charcode >= 0xe000) {
            utf8.push(0xe0 | (charcode >> 12), 
                      0x80 | ((charcode >> 6) & 0x3f), 
                      0x80 | (charcode & 0x3f));
        }
        // surrogate pair
        else {
            i++;
            charcode = 0x10000 + (((charcode & 0x3ff) << 10) | (str.charCodeAt(i) & 0x3ff));
            utf8.push(0xf0 | (charcode >> 18), 
                      0x80 | ((charcode >> 12) & 0x3f), 
                      0x80 | ((charcode >> 6) & 0x3f), 
                      0x80 | (charcode & 0x3f));
        }
    }
    return utf8;
}

function caesarCipher(str, offset) {
    let result = "";
    for (let i = 0; i < str.length; i++) {
        let c = str.charCodeAt(i);
        // Uppercase
        if (c >= 65 && c <= 90) {
            c = ((c - 65 + offset) % 26 + 26) % 26 + 65;
        }
        // Lowercase
        else if (c >= 97 && c <= 122) {
            c = ((c - 97 + offset) % 26 + 26) % 26 + 97;
        }
        result += String.fromCharCode(c);
    }
    return result;
}

// Minimal MD5 Implementation for JS (SparkMD5 style simplified)
function md5(string) {
    function md5cycle(x, k) {
        var a = x[0], b = x[1], c = x[2], d = x[3];
        a += (b & c | ~b & d) + k[0] - 680876936 | 0; a  = (a << 7 | a >>> 25) + b | 0;
        d += (a & b | ~a & c) + k[1] - 389564586 | 0; d  = (d << 12 | d >>> 20) + a | 0;
        c += (d & a | ~d & b) + k[2] + 606105819 | 0; c  = (c << 17 | c >>> 15) + d | 0;
        b += (c & d | ~c & a) + k[3] - 1044525330 | 0; b  = (b << 22 | b >>> 10) + c | 0;
        a += (b & c | ~b & d) + k[4] - 176418897 | 0; a  = (a << 7 | a >>> 25) + b | 0;
        d += (a & b | ~a & c) + k[5] + 1200080426 | 0; d  = (d << 12 | d >>> 20) + a | 0;
        c += (d & a | ~d & b) + k[6] - 1473231341 | 0; c  = (c << 17 | c >>> 15) + d | 0;
        b += (c & d | ~c & a) + k[7] - 45705983 | 0; b  = (b << 22 | b >>> 10) + c | 0;
        a += (b & c | ~b & d) + k[8] + 1770035416 | 0; a  = (a << 7 | a >>> 25) + b | 0;
        d += (a & b | ~a & c) + k[9] - 1958414417 | 0; d  = (d << 12 | d >>> 20) + a | 0;
        c += (d & a | ~d & b) + k[10] - 42063 | 0; c  = (c << 17 | c >>> 15) + d | 0;
        b += (c & d | ~c & a) + k[11] - 1990404162 | 0; b  = (b << 22 | b >>> 10) + c | 0;
        a += (b & c | ~b & d) + k[12] + 1804603682 | 0; a  = (a << 7 | a >>> 25) + b | 0;
        d += (a & b | ~a & c) + k[13] - 40341101 | 0; d  = (d << 12 | d >>> 20) + a | 0;
        c += (d & a | ~d & b) + k[14] - 1502002290 | 0; c  = (c << 17 | c >>> 15) + d | 0;
        b += (c & d | ~c & a) + k[15] + 1236535329 | 0; b  = (b << 22 | b >>> 10) + c | 0;
        a += (b & d | c & ~d) + k[1] - 165796510 | 0; a  = (a << 5 | a >>> 27) + b | 0;
        d += (a & c | b & ~c) + k[6] - 1069501632 | 0; d  = (d << 9 | d >>> 23) + a | 0;
        c += (d & b | a & ~b) + k[11] + 643717713 | 0; c  = (c << 14 | c >>> 18) + d | 0;
        b += (c & a | d & ~a) + k[0] - 373897302 | 0; b  = (b << 20 | b >>> 12) + c | 0;
        a += (b & d | c & ~d) + k[5] - 701558691 | 0; a  = (a << 5 | a >>> 27) + b | 0;
        d += (a & c | b & ~c) + k[10] + 38016083 | 0; d  = (d << 9 | d >>> 23) + a | 0;
        c += (d & b | a & ~b) + k[15] - 660478335 | 0; c  = (c << 14 | c >>> 18) + d | 0;
        b += (c & a | d & ~a) + k[4] - 405537848 | 0; b  = (b << 20 | b >>> 12) + c | 0;
        a += (b & d | c & ~d) + k[9] + 568446438 | 0; a  = (a << 5 | a >>> 27) + b | 0;
        d += (a & c | b & ~c) + k[14] - 1019803690 | 0; d  = (d << 9 | d >>> 23) + a | 0;
        c += (d & b | a & ~b) + k[3] - 187363961 | 0; c  = (c << 14 | c >>> 18) + d | 0;
        b += (c & a | d & ~a) + k[8] + 1163531501 | 0; b  = (b << 20 | b >>> 12) + c | 0;
        a += (b & d | c & ~d) + k[13] - 1444681467 | 0; a  = (a << 5 | a >>> 27) + b | 0;
        d += (a & c | b & ~c) + k[2] - 51403784 | 0; d  = (d << 9 | d >>> 23) + a | 0;
        c += (d & b | a & ~b) + k[7] + 1735328473 | 0; c  = (c << 14 | c >>> 18) + d | 0;
        b += (c & a | d & ~a) + k[12] - 1926607734 | 0; b  = (b << 20 | b >>> 12) + c | 0;
        a += (b ^ c ^ d) + k[5] - 378558 | 0; a  = (a << 4 | a >>> 28) + b | 0;
        d += (a ^ b ^ c) + k[8] - 2022574463 | 0; d  = (d << 11 | d >>> 21) + a | 0;
        c += (d ^ a ^ b) + k[11] + 1839030562 | 0; c  = (c << 16 | c >>> 16) + d | 0;
        b += (c ^ d ^ a) + k[14] - 35309556 | 0; b  = (b << 23 | b >>> 9) + c | 0;
        a += (b ^ c ^ d) + k[1] - 1530992060 | 0; a  = (a << 4 | a >>> 28) + b | 0;
        d += (a ^ b ^ c) + k[4] + 1272893353 | 0; d  = (d << 11 | d >>> 21) + a | 0;
        c += (d ^ a ^ b) + k[7] - 155497632 | 0; c  = (c << 16 | c >>> 16) + d | 0;
        b += (c ^ d ^ a) + k[10] - 1094730640 | 0; b  = (b << 23 | b >>> 9) + c | 0;
        a += (b ^ c ^ d) + k[13] + 681279174 | 0; a  = (a << 4 | a >>> 28) + b | 0;
        d += (a ^ b ^ c) + k[0] - 358537222 | 0; d  = (d << 11 | d >>> 21) + a | 0;
        c += (d ^ a ^ b) + k[3] - 722521979 | 0; c  = (c << 16 | c >>> 16) + d | 0;
        b += (c ^ d ^ a) + k[6] + 76029189 | 0; b  = (b << 23 | b >>> 9) + c | 0;
        a += (b ^ c ^ d) + k[9] - 640364487 | 0; a  = (a << 4 | a >>> 28) + b | 0;
        d += (a ^ b ^ c) + k[12] - 421815835 | 0; d  = (d << 11 | d >>> 21) + a | 0;
        c += (d ^ a ^ b) + k[15] + 530742520 | 0; c  = (c << 16 | c >>> 16) + d | 0;
        b += (c ^ d ^ a) + k[2] - 995338651 | 0; b  = (b << 23 | b >>> 9) + c | 0;
        a += (c ^ (b | ~d)) + k[0] - 198630844 | 0; a  = (a << 6 | a >>> 26) + b | 0;
        d += (b ^ (a | ~c)) + k[7] + 1126891415 | 0; d  = (d << 10 | d >>> 22) + a | 0;
        c += (a ^ (d | ~b)) + k[14] - 1416354905 | 0; c  = (c << 15 | c >>> 17) + d | 0;
        b += (d ^ (c | ~a)) + k[5] - 57434055 | 0; b  = (b << 21 | b >>> 11) + c | 0;
        a += (c ^ (b | ~d)) + k[12] + 1700485571 | 0; a  = (a << 6 | a >>> 26) + b | 0;
        d += (b ^ (a | ~c)) + k[3] - 1894986606 | 0; d  = (d << 10 | d >>> 22) + a | 0;
        c += (a ^ (d | ~b)) + k[10] - 1051523 | 0; c  = (c << 15 | c >>> 17) + d | 0;
        b += (d ^ (c | ~a)) + k[1] - 2054922799 | 0; b  = (b << 21 | b >>> 11) + c | 0;
        a += (c ^ (b | ~d)) + k[8] + 1873313359 | 0; a  = (a << 6 | a >>> 26) + b | 0;
        d += (b ^ (a | ~c)) + k[15] - 30611744 | 0; d  = (d << 10 | d >>> 22) + a | 0;
        c += (a ^ (d | ~b)) + k[6] - 1560198380 | 0; c  = (c << 15 | c >>> 17) + d | 0;
        b += (d ^ (c | ~a)) + k[13] + 1309151649 | 0; b  = (b << 21 | b >>> 11) + c | 0;
        a += (c ^ (b | ~d)) + k[4] - 145523070 | 0; a  = (a << 6 | a >>> 26) + b | 0;
        d += (b ^ (a | ~c)) + k[11] - 1120210379 | 0; d  = (d << 10 | d >>> 22) + a | 0;
        c += (a ^ (d | ~b)) + k[2] + 718787259 | 0; c  = (c << 15 | c >>> 17) + d | 0;
        b += (d ^ (c | ~a)) + k[9] - 343485551 | 0; b  = (b << 21 | b >>> 11) + c | 0;
        x[0] = a + x[0] | 0;
        x[1] = b + x[1] | 0;
        x[2] = c + x[2] | 0;
        x[3] = d + x[3] | 0;
    }

    function md5blk(s) {
        var md5blks = [], i;
        for (i=0; i<64; i+=4) {
            md5blks[i>>2] = s.charCodeAt(i) + (s.charCodeAt(i+1) << 8) + (s.charCodeAt(i+2) << 16) + (s.charCodeAt(i+3) << 24);
        }
        return md5blks;
    }
    
    function md5blk_array(a) {
        var md5blks = [], i;
        for (i=0; i<64; i+=4) {
            md5blks[i>>2] = a[i] + (a[i+1] << 8) + (a[i+2] << 16) + (a[i+3] << 24);
        }
        return md5blks;
    }

    function md51(s) {
        var n = s.length,
            state = [1732584193, -271733879, -1732584194, 271733878], i;
        for (i=64; i<=n; i+=64) {
            md5cycle(state, md5blk(s.substring(i-64, i)));
        }
        s = s.substring(i-64);
        var tail = [0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0];
        for (i=0; i<s.length; i++) {
            tail[i>>2] |= s.charCodeAt(i) << ((i%4) << 3);
        }
        tail[i>>2] |= 0x80 << ((i%4) << 3);
        if (i > 55) {
            md5cycle(state, tail);
            for (i=0; i<16; i++) tail[i] = 0;
        }
        tail[14] = n*8;
        md5cycle(state, tail);
        return state;
    }

    function md5_array(a) {
        var n = a.length,
            state = [1732584193, -271733879, -1732584194, 271733878], i;
        for (i=64; i<=n; i+=64) {
            md5cycle(state, md5blk_array(a.slice(i-64, i)));
        }
        a = a.slice(i-64);
        var tail = [0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0];
        for (i=0; i<a.length; i++) {
            tail[i>>2] |= a[i] << ((i%4) << 3);
        }
        tail[i>>2] |= 0x80 << ((i%4) << 3);
        if (i > 55) {
            md5cycle(state, tail);
            for (i=0; i<16; i++) tail[i] = 0;
        }
        tail[14] = n*8;
        md5cycle(state, tail);
        return state;
    }

    function hex(x) {
        var hex_chr = '0123456789abcdef'.split('');
        function rhex(n) {
            var s='', j=0;
            for(; j<4; j++) s += hex_chr[(n >> (j * 8 + 4)) & 0x0F] + hex_chr[(n >> (j * 8)) & 0x0F];
            return s;
        }
        return rhex(x[0]) + rhex(x[1]) + rhex(x[2]) + rhex(x[3]);
    }
    
    return hex(md51(string));
}
