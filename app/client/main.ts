/**
 * Hello world
 */

import {
  establishConnection,
  checkBoyncProgram,
  initBoyncAuction,
  checkBoyncAuction,
  preInitBoyncAuction,
  mintBoyncTokens,
  user1
} from './hello_world';

async function _mintBoyncTokens() {
  await mintBoyncTokens(user1, 1000000);
}

async function boync_auction_init() {
  console.log("Let's initialize Boync Auction account...");

  // Establish connection to the cluster
  await establishConnection();

  // Check if the program has been deployed
  await checkBoyncProgram();

  await preInitBoyncAuction();

  await initBoyncAuction();

  console.log('Success');
}

// boync_auction_init().then(
//   () => process.exit(),
//   err => {
//     console.error(err);
//     process.exit(-1);
//   },
// );

_mintBoyncTokens().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);
