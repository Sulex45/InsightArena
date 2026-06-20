import {
  MigrationInterface,
  QueryRunner,
  Table,
  TableIndex,
  TableForeignKey,
} from 'typeorm';

export class CreateCreatorEventPayout1776000000000 implements MigrationInterface {
  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.createTable(
      new Table({
        name: 'creator_event_payouts',
        columns: [
          {
            name: 'id',
            type: 'uuid',
            isPrimary: true,
            generationStrategy: 'uuid',
            default: 'uuid_generate_v4()',
          },
          {
            name: 'event_id',
            type: 'varchar',
            length: '255',
            isNullable: false,
          },
          {
            name: 'user_address',
            type: 'varchar',
            length: '255',
            isNullable: false,
          },
          {
            name: 'payout_amount_stroops',
            type: 'bigint',
            default: '0',
            isNullable: false,
          },
          {
            name: 'is_claimed',
            type: 'boolean',
            default: false,
            isNullable: false,
          },
          {
            name: 'leaderboard_entry_id',
            type: 'uuid',
            isNullable: false,
          },
          {
            name: 'created_at',
            type: 'timestamptz',
            default: 'CURRENT_TIMESTAMP',
            isNullable: false,
          },
        ],
      }),
      true,
    );

    // Composite unique: one payout row per participant per event
    await queryRunner.createIndex(
      'creator_event_payouts',
      new TableIndex({
        name: 'UQ_cep_event_address',
        columnNames: ['event_id', 'user_address'],
        isUnique: true,
      }),
    );

    // Supporting index for paginated list queries (GET /creator-events/:id/payouts)
    await queryRunner.createIndex(
      'creator_event_payouts',
      new TableIndex({
        name: 'IDX_cep_event_id',
        columnNames: ['event_id'],
      }),
    );

    // FK enforces referential integrity; CASCADE keeps payouts in sync if
    // a leaderboard entry is ever removed during a reindex/cleanup.
    await queryRunner.createForeignKey(
      'creator_event_payouts',
      new TableForeignKey({
        name: 'FK_cep_leaderboard_entry',
        columnNames: ['leaderboard_entry_id'],
        referencedTableName: 'creator_event_leaderboard_entries',
        referencedColumnNames: ['id'],
        onDelete: 'CASCADE',
      }),
    );
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.dropTable('creator_event_payouts');
  }
}
