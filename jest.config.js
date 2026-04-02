/** @type {import('jest').Config} */
export default {
  testMatch: ['<rootDir>/dist-test/tests/**/*.spec.js'],
  testPathIgnorePatterns: ['/node_modules/'],
  transform: {},
  coverageDirectory: 'coverage',
  coveragePathIgnorePatterns: ['/node_modules/', '/tests/'],
};
