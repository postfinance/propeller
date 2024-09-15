/**
 * @type {import('semantic-release').GlobalConfig}
 */
module.exports = {
  branches: ['main'],
  plugins: [
    '@semantic-release/commit-analyzer',
    '@semantic-release/release-notes-generator',
    [
      '@semantic-release/exec',
      {
        prepareCmd: '.github/pipeline/prepareCmd.sh ${nextRelease.version}',
        publishCmd: '.github/pipeline/dockerBuild.sh ${nextRelease.version}',
      },
    ],
    [
      '@semantic-release/github',
      {
        assets: [
          { path: 'target/release/propeller' },
          { path: 'target/release/propeller.md5' },
          { path: 'target/x86_64-pc-windows-gnu/release/propeller.exe' },
          { path: 'target/x86_64-pc-windows-gnu/release/propeller.exe.md5' },
        ],
      },
    ],
  ],
};
