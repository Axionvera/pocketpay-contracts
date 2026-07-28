import { describe, it, expect, beforeEach, vi } from 'vitest';
import { VaultService } from '../../src/services/vault.service';
import { TokenService } from '../../src/services/token.service';
import { AccountingService } from '../../src/services/accounting.service';

describe('Token Transfer Atomicity Tests', () => {
  let vaultService: VaultService;
  let tokenService: TokenService;
  let accountingService: AccountingService;

  // Mock state
  const mockVaultState = {
    totalBalance: '1000',
    userBalances: {
      'user1': '500',
      'user2': '300',
      'user3': '200',
    },
    lockRecords: {
      'lock1': {
        userId: 'user1',
        amount: '100',
        lockedAt: Date.now() - 1000,
        maturedAt: Date.now() + 10000,
        withdrawn: false,
      },
      'lock2': {
        userId: 'user2',
        amount: '200',
        lockedAt: Date.now() - 1000,
        maturedAt: Date.now() + 10000,
        withdrawn: false,
      },
    },
  };

  beforeEach(() => {
    // Reset mocks before each test
    vi.clearAllMocks();
  });

  describe('Deposit Atomicity', () => {
    it('should preserve state if deposit transfer fails', async () => {
      // Arrange
      const userId = 'user1';
      const depositAmount = '100';
      const initialUserBalance = mockVaultState.userBalances[userId];
      const initialVaultTotal = mockVaultState.totalBalance;

      // Mock token transfer to fail
      vi.spyOn(tokenService, 'transferTokens').mockRejectedValue(
        new Error('Token transfer failed')
      );

      // Mock accounting to not update (should be atomic)
      const accountingSpy = vi.spyOn(accountingService, 'updateBalance');

      // Act
      try {
        await vaultService.deposit(userId, depositAmount);
      } catch (error) {
        // Expected to fail
        expect(error.message).toContain('Token transfer failed');
      }

      // Assert
      // Verify user balance unchanged
      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(initialUserBalance);

      // Verify vault total unchanged
      const finalVaultTotal = await vaultService.getTotalBalance();
      expect(finalVaultTotal).toEqual(initialVaultTotal);

      // Verify accounting was NOT updated
      expect(accountingSpy).not.toHaveBeenCalled();

      // Verify lock records unchanged
      const lockRecords = await vaultService.getLockRecords(userId);
      expect(lockRecords).toEqual(mockVaultState.lockRecords);
    });

    it('should update state only after successful transfer', async () => {
      // Arrange
      const userId = 'user1';
      const depositAmount = '100';
      const initialUserBalance = mockVaultState.userBalances[userId];

      // Mock token transfer to succeed
      vi.spyOn(tokenService, 'transferTokens').mockResolvedValue({
        success: true,
        txHash: '0x123...',
      });

      const accountingSpy = vi.spyOn(accountingService, 'updateBalance');

      // Act
      await vaultService.deposit(userId, depositAmount);

      // Assert
      // Verify accounting was updated
      expect(accountingSpy).toHaveBeenCalledWith(
        userId,
        depositAmount,
        'deposit'
      );

      // Verify user balance updated
      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(
        (BigInt(initialUserBalance) + BigInt(depositAmount)).toString()
      );
    });

    it('should not create lock record if deposit fails', async () => {
      // Arrange
      const userId = 'user1';
      const depositAmount = '100';

      vi.spyOn(tokenService, 'transferTokens').mockRejectedValue(
        new Error('Token transfer failed')
      );

      const createLockSpy = vi.spyOn(vaultService, 'createLockRecord');

      // Act
      try {
        await vaultService.deposit(userId, depositAmount);
      } catch (error) {
        // Expected
      }

      // Assert
      expect(createLockSpy).not.toHaveBeenCalled();
    });
  });

  describe('Withdrawal Atomicity', () => {
    it('should preserve state if withdrawal transfer fails', async () => {
      // Arrange
      const userId = 'user1';
      const withdrawalAmount = '100';
      const initialUserBalance = mockVaultState.userBalances[userId];
      const initialVaultTotal = mockVaultState.totalBalance;

      vi.spyOn(tokenService, 'transferTokens').mockRejectedValue(
        new Error('Token transfer failed')
      );

      const accountingSpy = vi.spyOn(accountingService, 'updateBalance');

      // Act
      try {
        await vaultService.withdraw(userId, withdrawalAmount);
      } catch (error) {
        expect(error.message).toContain('Token transfer failed');
      }

      // Assert
      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(initialUserBalance);

      const finalVaultTotal = await vaultService.getTotalBalance();
      expect(finalVaultTotal).toEqual(initialVaultTotal);

      expect(accountingSpy).not.toHaveBeenCalled();
    });

    it('should update state only after successful withdrawal transfer', async () => {
      // Arrange
      const userId = 'user1';
      const withdrawalAmount = '100';
      const initialUserBalance = mockVaultState.userBalances[userId];

      vi.spyOn(tokenService, 'transferTokens').mockResolvedValue({
        success: true,
        txHash: '0x456...',
      });

      const accountingSpy = vi.spyOn(accountingService, 'updateBalance');

      // Act
      await vaultService.withdraw(userId, withdrawalAmount);

      // Assert
      expect(accountingSpy).toHaveBeenCalledWith(
        userId,
        withdrawalAmount,
        'withdraw'
      );

      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(
        (BigInt(initialUserBalance) - BigInt(withdrawalAmount)).toString()
      );
    });
  });

  describe('Matured Lock Withdrawal Atomicity', () => {
    it('should preserve state if matured lock withdrawal fails', async () => {
      // Arrange
      const lockId = 'lock1';
      const userId = 'user1';
      const lockAmount = mockVaultState.lockRecords[lockId].amount;
      const initialUserBalance = mockVaultState.userBalances[userId];
      const initialLockRecord = { ...mockVaultState.lockRecords[lockId] };

      vi.spyOn(tokenService, 'transferTokens').mockRejectedValue(
        new Error('Token transfer failed')
      );

      const updateLockSpy = vi.spyOn(vaultService, 'updateLockRecord');

      // Act
      try {
        await vaultService.withdrawMaturedLock(userId, lockId);
      } catch (error) {
        expect(error.message).toContain('Token transfer failed');
      }

      // Assert
      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(initialUserBalance);

      // Verify lock record unchanged
      const finalLockRecord = await vaultService.getLockRecord(lockId);
      expect(finalLockRecord.withdrawn).toBe(initialLockRecord.withdrawn);
      expect(finalLockRecord.amount).toBe(initialLockRecord.amount);

      expect(updateLockSpy).not.toHaveBeenCalled();
    });

    it('should update lock record only after successful transfer', async () => {
      // Arrange
      const lockId = 'lock1';
      const userId = 'user1';

      vi.spyOn(tokenService, 'transferTokens').mockResolvedValue({
        success: true,
        txHash: '0x789...',
      });

      const updateLockSpy = vi.spyOn(vaultService, 'updateLockRecord');

      // Act
      await vaultService.withdrawMaturedLock(userId, lockId);

      // Assert
      expect(updateLockSpy).toHaveBeenCalledWith(lockId, {
        withdrawn: true,
        withdrawnAt: expect.any(Number),
      });
    });

    it('should not mark lock as withdrawn if transfer fails', async () => {
      // Arrange
      const lockId = 'lock1';
      const userId = 'user1';

      vi.spyOn(tokenService, 'transferTokens').mockRejectedValue(
        new Error('Token transfer failed')
      );

      const updateLockSpy = vi.spyOn(vaultService, 'updateLockRecord');

      // Act
      try {
        await vaultService.withdrawMaturedLock(userId, lockId);
      } catch (error) {
        // Expected
      }

      // Assert
      expect(updateLockSpy).not.toHaveBeenCalled();
    });
  });

  describe('Vault Consistency', () => {
    it('should maintain vault total consistency across operations', async () => {
      // Arrange
      const operations = [
        { type: 'deposit', userId: 'user1', amount: '100' },
        { type: 'withdraw', userId: 'user2', amount: '50' },
        { type: 'deposit', userId: 'user3', amount: '75' },
      ];

      const initialTotal = mockVaultState.totalBalance;
      let currentTotal = BigInt(initialTotal);

      // Mock token transfers to succeed
      vi.spyOn(tokenService, 'transferTokens').mockResolvedValue({
        success: true,
        txHash: '0xabc...',
      });

      vi.spyOn(accountingService, 'updateBalance').mockImplementation(
        (userId, amount, type) => {
          if (type === 'deposit') {
            currentTotal += BigInt(amount);
          } else if (type === 'withdraw') {
            currentTotal -= BigInt(amount);
          }
        }
      );

      // Act
      for (const op of operations) {
        if (op.type === 'deposit') {
          await vaultService.deposit(op.userId, op.amount);
        } else {
          await vaultService.withdraw(op.userId, op.amount);
        }
      }

      // Assert
      const finalTotal = await vaultService.getTotalBalance();
      expect(finalTotal).toEqual(currentTotal.toString());
    });

    it('should rollback accounting if transfer fails mid-operation', async () => {
      // Arrange
      const userId = 'user1';
      const amount = '100';

      // Simulate transfer failure after accounting update
      vi.spyOn(accountingService, 'updateBalance').mockImplementation(() => {
        // Accounting updated
      });

      vi.spyOn(tokenService, 'transferTokens').mockRejectedValue(
        new Error('Transfer failed')
      );

      // Act
      try {
        await vaultService.deposit(userId, amount);
      } catch (error) {
        // Expected
      }

      // Assert
      // Verify vault total returned to previous state
      const finalTotal = await vaultService.getTotalBalance();
      expect(finalTotal).toEqual(mockVaultState.totalBalance);

      // Verify user balance returned to previous state
      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(mockVaultState.userBalances[userId]);
    });
  });

  describe('Error Recovery', () => {
    it('should handle partial failures gracefully', async () => {
      // Arrange
      const userId = 'user1';
      const amount = '100';

      // Simulate network failure during transfer
      let callCount = 0;
      vi.spyOn(tokenService, 'transferTokens').mockImplementation(async () => {
        callCount++;
        if (callCount === 1) {
          throw new Error('Network timeout');
        }
        return { success: true, txHash: '0xdef...' };
      });

      const accountingSpy = vi.spyOn(accountingService, 'updateBalance');

      // Act - retry should succeed
      await vaultService.deposit(userId, amount);

      // Assert
      expect(accountingSpy).toHaveBeenCalledTimes(1);
      const finalUserBalance = await vaultService.getUserBalance(userId);
      expect(finalUserBalance).toEqual(
        (BigInt(mockVaultState.userBalances[userId]) + BigInt(amount)).toString()
      );
    });
  });
});
