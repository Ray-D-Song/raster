const originalDirname = __dirname;
const originalFilename = __filename;

__dirname = "/mutated-dirname";
__filename = "/mutated-filename";

module.exports = {
  originalDirname,
  originalFilename,
  mutatedDirname: __dirname,
  mutatedFilename: __filename,
};
