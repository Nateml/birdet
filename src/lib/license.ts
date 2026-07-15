// Helpers for crediting Xeno-Canto recordings under their Creative Commons
// licences. XC gives us the recordist and a licence URL; the store also keeps
// the XC id (which is both a stable credit and a link back to the source page).

// Turn a Creative Commons licence URL into a short human label, e.g.
//   //creativecommons.org/licenses/by-nc-sa/4.0/  → "CC BY-NC-SA 4.0"
//   //creativecommons.org/publicdomain/zero/1.0/  → "CC0 1.0"
// Returns null for an empty/unrecognised URL so callers can hide the chip.
export function licenseLabel(url: string | null | undefined): string | null {
    if (!url) return null;
    const path = url.replace(/^https?:/, '').replace(/^\/\//, '');
    const m = path.match(/creativecommons\.org\/licenses\/([a-z-]+)\/([0-9.]+)/i);
    if (m) return `CC ${m[1].toUpperCase()} ${m[2]}`;
    const zero = path.match(/creativecommons\.org\/publicdomain\/zero\/([0-9.]+)/i);
    if (zero) return `CC0 ${zero[1]}`;
    if (/creativecommons\.org\/publicdomain\/mark/i.test(path)) return 'Public Domain';
    return null;
}

// Normalise a licence URL to an absolute https link (XC returns protocol-relative
// "//creativecommons.org/…" URLs).
export function licenseHref(url: string | null | undefined): string | null {
    if (!url) return null;
    if (url.startsWith('//')) return `https:${url}`;
    if (url.startsWith('http')) return url;
    return `https://${url}`;
}

// The Xeno-Canto page for a recording id.
export function xcUrl(xcId: string | null | undefined): string | null {
    return xcId ? `https://xeno-canto.org/${xcId}` : null;
}
