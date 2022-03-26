import { createHash } from "crypto";
const name = "istanbul-oxi-instrument";
const VERSION = "4";

const SHA = "sha1";
module.exports = {
  SHA,
  MAGIC_KEY: "_coverageSchema",
  MAGIC_VALUE: createHash(SHA)
    .update(name + "@" + VERSION)
    .digest("hex"),
};
