import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { AccountController } from './account.controller';
import { AccountService } from './account.service';
import { DataExportJob } from './entities/data-export-job.entity';

@Module({
  imports: [TypeOrmModule.forFeature([DataExportJob])],
  controllers: [AccountController],
  providers: [AccountService],
})
export class AccountModule {}
