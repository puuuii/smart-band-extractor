import asyncio
from bleak import BleakScanner


async def main():
    print("BLEスキャン開始（30秒）...")

    devices = await BleakScanner.discover(
        timeout=30.0,
        return_adv=True,
    )

    print(f"\n検出数: {len(devices)}")

    for address, (device, adv) in devices.items():
        print("\n" + "=" * 50)
        print(f"Address       : {address}")
        print(f"Device name   : {device.name!r}")
        print(f"Local name    : {adv.local_name!r}")
        print(f"RSSI          : {adv.rssi}")
        print(f"Service UUIDs : {adv.service_uuids}")

        print("Manufacturer data:")
        for key, value in adv.manufacturer_data.items():
            print(f"  {key}: {value.hex()}")

        print("Service data:")
        for key, value in adv.service_data.items():
            print(f"  {key}: {value.hex()}")


if __name__ == "__main__":
    asyncio.run(main())
