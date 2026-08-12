export function isSafeExternalHttpsUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "https:" && !isLocalOrPrivateHost(url.hostname);
  } catch {
    return false;
  }
}

export function isLocalOrPrivateHost(hostname: string): boolean {
  const host = hostname
    .toLowerCase()
    .replace(/^\[|\]$/g, "")
    .replace(/\.$/, "");
  if (
    host === "localhost" ||
    host.endsWith(".localhost") ||
    ["local", "lan", "home", "internal", "corp"].some(
      (suffix) => host === suffix || host.endsWith(`.${suffix}`),
    )
  ) {
    return true;
  }

  if (host.includes(":")) {
    if (host.startsWith("::ffff:")) {
      const mapped = host.slice("::ffff:".length);
      if (mapped.includes(".")) return isLocalOrPrivateHost(mapped);
      const [upper, lower] = mapped.split(":");
      if (upper && lower && /^[0-9a-f]{1,4}$/.test(upper) && /^[0-9a-f]{1,4}$/.test(lower)) {
        const high = Number.parseInt(upper, 16);
        const low = Number.parseInt(lower, 16);
        return isLocalOrPrivateHost(
          `${high >> 8}.${high & 0xff}.${low >> 8}.${low & 0xff}`,
        );
      }
    }
    return (
      host === "::" ||
      host === "::1" ||
      host.startsWith("ff") ||
      /^f[cd][0-9a-f:]*$/.test(host) ||
      /^fe[89ab][0-9a-f:]*$/.test(host)
    );
  }

  const labels = host.split(".");
  return labels.some((_, index) =>
    isNonPublicIpv4(labels.slice(index, index + 4)),
  );
}

function isNonPublicIpv4(labels: string[]): boolean {
  if (
    labels.length !== 4 ||
    labels.some((label) => !/^\d+$/.test(label))
  ) {
    return false;
  }
  const octets = labels.map(Number);
  if (octets.some((octet) => octet > 255)) return false;
  const first = octets[0] ?? -1;
  const second = octets[1] ?? -1;
  return (
    first === 0 ||
    first === 10 ||
    first === 127 ||
    first === 169 && second === 254 ||
    first === 172 && second >= 16 && second <= 31 ||
    first === 192 && second === 168 ||
    first === 100 && second >= 64 && second <= 127 ||
    first >= 224
  );
}
