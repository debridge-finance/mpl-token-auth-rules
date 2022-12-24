/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet';
/**
 * This type is used to derive the {@link CreateOrUpdateArgs} type as well as the de/serializer.
 * However don't refer to it in your code but use the {@link CreateOrUpdateArgs} type instead.
 *
 * @category userTypes
 * @category enums
 * @category generated
 * @private
 */
export type CreateOrUpdateArgsRecord = {
  V1: { serializedRuleSet: Uint8Array };
};

/**
 * Union type respresenting the CreateOrUpdateArgs data enum defined in Rust.
 *
 * NOTE: that it includes a `__kind` property which allows to narrow types in
 * switch/if statements.
 * Additionally `isCreateOrUpdateArgs*` type guards are exposed below to narrow to a specific variant.
 *
 * @category userTypes
 * @category enums
 * @category generated
 */
export type CreateOrUpdateArgs = beet.DataEnumKeyAsKind<CreateOrUpdateArgsRecord>;

export const isCreateOrUpdateArgsV1 = (
  x: CreateOrUpdateArgs,
): x is CreateOrUpdateArgs & { __kind: 'V1' } => x.__kind === 'V1';

/**
 * @category userTypes
 * @category generated
 */
export const createOrUpdateArgsBeet = beet.dataEnum<CreateOrUpdateArgsRecord>([
  [
    'V1',
    new beet.FixableBeetArgsStruct<CreateOrUpdateArgsRecord['V1']>(
      [['serializedRuleSet', beet.bytes]],
      'CreateOrUpdateArgsRecord["V1"]',
    ),
  ],
]) as beet.FixableBeet<CreateOrUpdateArgs, CreateOrUpdateArgs>;
