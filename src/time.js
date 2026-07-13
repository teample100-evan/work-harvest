import { randomBytes } from "node:crypto";

function partsFor(timestamp, timezone) {
  const date = new Date(timestamp);
  if (Number.isNaN(date.valueOf())) {
    throw new Error(`Invalid timestamp: ${timestamp}`);
  }

  return Object.fromEntries(
    new Intl.DateTimeFormat("en-CA", {
      timeZone: timezone,
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hourCycle: "h23",
    })
      .formatToParts(date)
      .filter((part) => part.type !== "literal")
      .map((part) => [part.type, part.value]),
  );
}

export function calendarParts(timestamp, timezone = "Asia/Seoul") {
  const { year, month, day, hour, minute, second } = partsFor(
    timestamp,
    timezone,
  );
  return { year, month, day, hour, minute, second };
}

export function calendarDate(timestamp, timezone = "Asia/Seoul") {
  const { year, month, day } = calendarParts(timestamp, timezone);
  return `${year}-${month}-${day}`;
}

export function generateCheckpointId(timestamp, timezone = "Asia/Seoul") {
  const { year, month, day, hour, minute, second } = calendarParts(
    timestamp,
    timezone,
  );
  const suffix = randomBytes(3).toString("hex");
  return `CP-${year}${month}${day}-${hour}${minute}${second}-${suffix}`;
}
