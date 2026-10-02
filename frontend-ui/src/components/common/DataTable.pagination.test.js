import { afterEach, expect, test } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { ElTable, ElTableColumn } from 'element-plus/es/components/table/index'
import { ElPagination } from 'element-plus/es/components/pagination/index'
import { ElButton } from 'element-plus/es/components/button/index'
import { ElIcon } from 'element-plus/es/components/icon/index'
import DataTable from './DataTable.vue'
import Pagination from './Pagination.vue'

let wrapper
afterEach(() => wrapper?.unmount())

for (const size of [20, 50]) {
  test(`server pagination preserves ${size} rows across next-page interaction`, async () => {
    wrapper = mount(DataTable, {
      props: {
        data: [{ id: 'record' }], columns: [{ prop: 'id', label: 'ID' }],
        total: 130, serverPagination: true, initialPageSize: size,
      },
      global: { components: { ElTable, ElTableColumn, ElPagination, ElButton, ElIcon } },
    })
    await flushPromises()
    expect(wrapper.findComponent(Pagination).props('pageSize')).toBe(size)
    await wrapper.find('button.btn-next').trigger('click')
    await flushPromises()
    expect(wrapper.emitted('page-change').at(-1)).toEqual([{ page: 2, size }])
  })
}
