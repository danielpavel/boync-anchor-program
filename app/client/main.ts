/**
 * Hello world
 */

import {
  establishConnection,
  checkBoyncProgram,
  initBoyncAuction,
  checkBoyncAuction,
  preInitBoyncAuction,
} from './hello_world';

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

boync_auction_init().then(
  () => process.exit(),
  err => {
    console.error(err);
    process.exit(-1);
  },
);
