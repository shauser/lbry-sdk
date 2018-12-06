import logging
import asyncio
from itertools import chain
from torba.testcase import IntegrationTestCase
from torba.client.constants import COIN


class BasicTransactionTests(IntegrationTestCase):

    VERBOSITY = logging.WARN

    async def test_stressing(self):
        await self.blockchain.generate(1000)
        await self.assertBalance(self.account, '0.0')
        addresses = await self.account.receiving.get_addresses()

        sends = list(chain(
            (self.blockchain.send_to_address(address, 10) for address in addresses),
            (self.blockchain.send_to_address(addresses[-1], 10) for _ in range(10))
        ))

        for batch in range(0, len(sends), 10):
            txids = await asyncio.gather(*sends[batch:batch+10])
            await asyncio.wait([
                self.on_transaction_id(txid) for txid in txids
            ])

        await self.assertBalance(self.account, '300.0')
        addresses = await self.account.receiving.get_addresses()
        self.assertEqual(40, len(addresses))

        await self.blockchain.generate(1)

        self.assertEqual(30, await self.account.get_utxo_count())

        hash1 = self.ledger.address_to_hash160(addresses[-1])
        tx = await self.ledger.transaction_class.create(
            [],
            [self.ledger.transaction_class.output_class.pay_pubkey_hash(299*COIN, hash1)],
            [self.account], self.account
        )
        await self.broadcast(tx)
        await self.ledger.wait(tx)

        self.assertEqual(2, await self.account.get_utxo_count())  # 299 + change

    async def test_sending_and_receiving(self):
        account1, account2 = self.account, self.wallet.generate_account(self.ledger)
        await self.ledger.subscribe_account(account2)

        await self.assertBalance(account1, '0.0')
        await self.assertBalance(account2, '0.0')

        sendtxids = []
        for i in range(5):
            address1 = await account1.receiving.get_or_create_usable_address()
            sendtxid = await self.blockchain.send_to_address(address1, 1.1)
            sendtxids.append(sendtxid)
            await self.on_transaction_id(sendtxid)  # mempool
        await self.blockchain.generate(1)
        await asyncio.wait([  # confirmed
            self.on_transaction_id(txid) for txid in sendtxids
        ])

        await self.assertBalance(account1, '5.5')
        await self.assertBalance(account2, '0.0')

        address2 = await account2.receiving.get_or_create_usable_address()
        hash2 = self.ledger.address_to_hash160(address2)
        tx = await self.ledger.transaction_class.create(
            [],
            [self.ledger.transaction_class.output_class.pay_pubkey_hash(2*COIN, hash2)],
            [account1], account1
        )
        await self.broadcast(tx)
        await self.ledger.wait(tx)  # mempool
        await self.blockchain.generate(1)
        await self.ledger.wait(tx)  # confirmed

        await self.assertBalance(account1, '3.499802')
        await self.assertBalance(account2, '2.0')

        utxos = await self.account.get_utxos()
        tx = await self.ledger.transaction_class.create(
            [self.ledger.transaction_class.input_class.spend(utxos[0])],
            [],
            [account1], account1
        )
        await self.broadcast(tx)
        await self.ledger.wait(tx)  # mempool
        await self.blockchain.generate(1)
        await self.ledger.wait(tx)  # confirmed

        txs = await account1.get_transactions()
        tx = txs[1]
        self.assertEqual(round(tx.inputs[0].txo_ref.txo.amount/COIN, 1), 1.1)
        self.assertEqual(round(tx.inputs[1].txo_ref.txo.amount/COIN, 1), 1.1)
        self.assertEqual(round(tx.outputs[0].amount/COIN, 1), 2.0)
        self.assertEqual(tx.outputs[0].get_address(self.ledger), address2)
        self.assertEqual(tx.outputs[0].is_change, False)
        self.assertEqual(tx.outputs[1].is_change, True)
